use mlua::{Function, Lua, RegistryKey, Table};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use crate::error::DatabaseError;
use crate::storage::Storage;

struct Extension {
    key: RegistryKey,
    prefix: String,
}

pub struct ExtensionManager {
    lua: Lua,
    extensions: Vec<Extension>,
}

impl ExtensionManager {
    pub fn load(dir_name: &str) -> Self {
        let lua = Lua::new();
        let mut extensions = Vec::new();
        let dir = Path::new(dir_name);

        if !dir.is_dir() {
            return Self { lua, extensions };
        }

        let entries = match fs::read_dir(dir) {
            Ok(en) => en,
            Err(e) => {
                eprintln!("Error reading extensions directory: {e}");
                return Self { lua, extensions };
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("lua") {
                continue;
            }

            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let default_prefix = format!("{stem}_");

            if let Ok(source) = fs::read_to_string(&path) {
                if source.trim().is_empty() {
                    continue;
                }

                if let Ok(table) = lua.load(&source).set_name(&name).eval::<Table>() {
                    let prefix = table.get::<String>("prefix").unwrap_or(default_prefix);
                    if let Ok(key) = lua.create_registry_value(table) {
                        extensions.push(Extension { key, prefix });
                    }
                }
            }
        }

        Self { lua, extensions }
    }

    pub fn trigger_pre_hook(
        &self,
        storage: &Rc<RefCell<Storage>>,
        command: &str,
        key: Option<&str>,
        value: Option<&str>,
    ) -> Result<Option<String>, DatabaseError> {
        let key = match key {
            Some(k) => k,
            None => return Ok(value.map(str::to_string)),
        };

        let matching_extensions: Vec<_> = self
            .extensions
            .iter()
            .filter(|ext| key.starts_with(&ext.prefix))
            .collect();

        if matching_extensions.is_empty() {
            return Ok(value.map(str::to_string));
        }

        let ctx = self
            .create_context_table(storage, command, Some(key), value, None)
            .map_err(|e| clean_lua_error(&e))?;

        for ext in matching_extensions {
            if let Ok(table) = self.lua.registry_value::<Table>(&ext.key)
                && let Ok(hook) = table.get::<Function>("pre_hook")
            {
                hook.call::<()>(ctx.clone())
                    .map_err(|e| clean_lua_error(&e))?;
            }
        }

        let new_value = ctx
            .get::<Option<String>>("value")
            .unwrap_or_else(|_| value.map(str::to_string));

        Ok(new_value)
    }

    pub fn trigger_post_hook(
        &self,
        storage: &Rc<RefCell<Storage>>,
        command: &str,
        key: Option<&str>,
        value: Option<&str>,
        result: Option<&str>,
    ) -> Result<Option<String>, DatabaseError> {
        let key = match key {
            Some(k) => k,
            None => return Ok(result.map(str::to_string)),
        };

        let matching_extensions: Vec<_> = self
            .extensions
            .iter()
            .filter(|ext| key.starts_with(&ext.prefix))
            .collect();

        if matching_extensions.is_empty() {
            return Ok(result.map(str::to_string));
        }

        let ctx = self
            .create_context_table(storage, command, Some(key), value, result)
            .map_err(|e| clean_lua_error(&e))?;

        for ext in matching_extensions {
            if let Ok(table) = self.lua.registry_value::<Table>(&ext.key)
                && let Ok(hook) = table.get::<Function>("post_hook")
            {
                hook.call::<()>(ctx.clone())
                    .map_err(|e| clean_lua_error(&e))?;
            }
        }

        let new_result = ctx
            .get::<Option<String>>("result")
            .unwrap_or_else(|_| result.map(str::to_string));

        Ok(new_result)
    }

    fn create_context_table(
        &self,
        storage: &Rc<RefCell<Storage>>,
        command: &str,
        key: Option<&str>,
        value: Option<&str>,
        result: Option<&str>,
    ) -> mlua::Result<Table> {
        let table = self.lua.create_table()?;
        table.set("command", command)?;
        if let Some(key) = key {
            table.set("key", key)?;
        }
        if let Some(value) = value {
            table.set("value", value)?;
        }
        if let Some(result) = result {
            table.set("result", result)?;
        }

        // Generic query by key
        let storage_get = Rc::clone(storage);
        let get_fn = self.lua.create_function(move |_, k: String| {
            let val = storage_get.borrow().get(&k).map(str::to_string);
            Ok(val)
        })?;
        table.set("get", get_fn)?;

        // Generic reverse query: find key by value (O(1))
        let storage_find = Rc::clone(storage);
        let get_key_by_val_fn = self.lua.create_function(move |_, v: String| {
            let k = storage_find.borrow().get_key_by_value(&v).map(str::to_string);
            Ok(k)
        })?;
        table.set("get_key_by_value", get_key_by_val_fn)?;

        Ok(table)
    }
}

fn extract_clean_message(raw: &str) -> String {
    let without_traceback = raw.split("\nstack traceback:").next().unwrap_or(raw);
    let first_line = without_traceback
        .lines()
        .next()
        .unwrap_or(without_traceback)
        .trim();

    let s = first_line
        .strip_prefix("runtime error: ")
        .unwrap_or(first_line)
        .trim();

    let message = if let Some((prefix, msg)) = s.split_once(": ") {
        if let Some((_, line_num)) = prefix.rsplit_once(':') {
            if !line_num.is_empty() && line_num.chars().all(|c| c.is_ascii_digit()) {
                msg.trim()
            } else {
                s
            }
        } else {
            s
        }
    } else {
        s
    };

    let trimmed = message
        .strip_prefix("ERRO: ")
        .or_else(|| message.strip_prefix("error: "))
        .unwrap_or(message)
        .trim();

    trimmed.to_string()
}

fn clean_lua_error(err: &mlua::Error) -> DatabaseError {
    match err {
        mlua::Error::CallbackError { cause, .. } => clean_lua_error(cause),
        mlua::Error::RuntimeError(msg) => DatabaseError::Extension(extract_clean_message(msg)),
        other => DatabaseError::Extension(extract_clean_message(&other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_clean_message() {
        let msg1 = "[string \"cpf.lua\"]:8: CPF inválido\nstack traceback:\n  [C]: in ?";
        assert_eq!(extract_clean_message(msg1), "CPF inválido");

        let msg2 = "runtime error: [string \"cpf.lua\"]:12: Dígito incorreto";
        assert_eq!(extract_clean_message(msg2), "Dígito incorreto");

        let msg3 = "extensions/data.lua:5: Mensagem sem string prefix";
        assert_eq!(extract_clean_message(msg3), "Mensagem sem string prefix");

        let msg4 = "Erro simples sem prefixo";
        assert_eq!(extract_clean_message(msg4), "Erro simples sem prefixo");

        let msg5 = "runtime error: ERRO: já prefixado";
        assert_eq!(extract_clean_message(msg5), "já prefixado");
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn test_clean_lua_error_types() {
        let rt_err = mlua::Error::RuntimeError("[string \"test.lua\"]:1: falha crítica".to_string());
        assert_eq!(
            clean_lua_error(&rt_err),
            DatabaseError::Extension("falha crítica".to_string())
        );

        let cb_err = mlua::Error::CallbackError {
            traceback: "stack traceback: ...".to_string(),
            cause: std::sync::Arc::new(mlua::Error::RuntimeError(
                "[string \"test.lua\"]:2: falha interna".to_string(),
            )),
        };
        assert_eq!(
            clean_lua_error(&cb_err),
            DatabaseError::Extension("falha interna".to_string())
        );
    }

    #[test]
    fn test_cpf_extension_validation_and_formatting() {
        let ext_manager = ExtensionManager::load("extensions");
        let storage = Rc::new(RefCell::new(Storage::new()));

        // Valid CPF insertion
        let pre_res = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("cpf_test"),
            Some("12345678909"),
        );
        assert!(pre_res.is_ok());
        storage.borrow_mut().add("cpf_test", "12345678909");

        // Format on GET
        let post_res = ext_manager.trigger_post_hook(
            &storage,
            "GET",
            Some("cpf_test"),
            None,
            Some("12345678909"),
        );
        assert_eq!(post_res.unwrap(), Some("123.456.789-09".to_string()));

        // Invalid CPF length
        let err_len = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("cpf_test2"),
            Some("1234567890"),
        );
        assert!(err_len.is_err());

        // Repeated digits
        let err_rep = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("cpf_test3"),
            Some("11111111111"),
        );
        assert!(err_rep.is_err());

        // Invalid checksum
        let err_check = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("cpf_test4"),
            Some("12345678900"),
        );
        assert!(err_check.is_err());

        // Unicity: duplicate CPF on another key
        let err_dup = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("cpf_other"),
            Some("12345678909"),
        );
        assert!(err_dup.is_err());

        // Overwrite same key with same CPF allowed
        let ok_overwrite = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("cpf_test"),
            Some("12345678909"),
        );
        assert!(ok_overwrite.is_ok());
    }

    #[test]
    fn test_data_extension_validation_and_formatting() {
        let ext_manager = ExtensionManager::load("extensions");
        let storage = Rc::new(RefCell::new(Storage::new()));

        // Valid date
        let pre_ok = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("data_test"),
            Some("2024-02-29"),
        );
        assert!(pre_ok.is_ok());
        storage.borrow_mut().add("data_test", "2024-02-29");

        // Format on GET
        let post_ok = ext_manager.trigger_post_hook(
            &storage,
            "GET",
            Some("data_test"),
            None,
            Some("2024-02-29"),
        );
        assert_eq!(post_ok.unwrap(), Some("29/02/2024".to_string()));

        // Century leap year 2000
        let pre_2000 = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("data_2000"),
            Some("2000-02-29"),
        );
        assert!(pre_2000.is_ok());

        // Century non-leap year 1900
        let pre_1900 = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("data_1900"),
            Some("1900-02-29"),
        );
        assert!(pre_1900.is_err());

        // Century non-leap year 2100
        let pre_2100 = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("data_2100"),
            Some("2100-02-29"),
        );
        assert!(pre_2100.is_err());

        // Invalid month
        let pre_inv_month = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("data_bad"),
            Some("2023-13-01"),
        );
        assert!(pre_inv_month.is_err());

        // Invalid day
        let pre_inv_day = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("data_bad2"),
            Some("2023-04-31"),
        );
        assert!(pre_inv_day.is_err());

        // Loose format
        let pre_loose = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("data_bad3"),
            Some("2023-1-5"),
        );
        assert!(pre_loose.is_err());
    }

    #[test]
    fn test_json_extension_crud_and_patching() {
        let ext_manager = ExtensionManager::load("extensions");
        let storage = Rc::new(RefCell::new(Storage::new()));

        // 1. Full JSON creation
        let pre_init = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("json_user"),
            Some("{\"name\": \"Alice\", \"age\": 30, \"active\": true}"),
        );
        assert!(pre_init.is_ok());
        let val1 = pre_init.unwrap().unwrap();
        assert_eq!(val1, "{\"active\":true,\"age\":30,\"name\":\"Alice\"}");
        storage.borrow_mut().add("json_user", &val1);

        // GET formatting
        let post_get1 = ext_manager.trigger_post_hook(
            &storage,
            "GET",
            Some("json_user"),
            None,
            Some(&val1),
        );
        assert_eq!(
            post_get1.unwrap(),
            Some("+--------+---------+\n| CAMPO  | VALOR   |\n+--------+---------+\n| active | true    |\n| age    | 30      |\n| name   | \"Alice\" |\n+--------+---------+".to_string())
        );

        // 2. Patch field: @age=31
        let pre_patch1 = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("json_user"),
            Some("@age=31"),
        );
        assert!(pre_patch1.is_ok());
        let val2 = pre_patch1.unwrap().unwrap();
        assert_eq!(val2, "{\"active\":true,\"age\":31,\"name\":\"Alice\"}");
        storage.borrow_mut().add("json_user", &val2);

        // 3. Patch add field: +@city="Sao Paulo"
        let pre_patch2 = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("json_user"),
            Some("+@city=\"Sao Paulo\""),
        );
        assert!(pre_patch2.is_ok());
        let val3 = pre_patch2.unwrap().unwrap();
        assert_eq!(
            val3,
            "{\"active\":true,\"age\":31,\"city\":\"Sao Paulo\",\"name\":\"Alice\"}"
        );
        storage.borrow_mut().add("json_user", &val3);

        // 4. Patch remove field: -@age
        let pre_patch3 = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("json_user"),
            Some("-@age"),
        );
        assert!(pre_patch3.is_ok());
        let val4 = pre_patch3.unwrap().unwrap();
        assert_eq!(
            val4,
            "{\"active\":true,\"city\":\"Sao Paulo\",\"name\":\"Alice\"}"
        );
        storage.borrow_mut().add("json_user", &val4);

        // 5. Idempotent patch: remove nonexistent field (no error)
        let ok_del = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("json_user"),
            Some("-@age"),
        );
        assert!(ok_del.is_ok());

        // 6. Idempotent patch: add field on new key (creates document)
        let ok_new = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("json_new"),
            Some("+@status=\"ativo\""),
        );
        assert!(ok_new.is_ok());
        assert_eq!(ok_new.unwrap().unwrap(), "{\"status\":\"ativo\"}");

        // 7. Invalid JSON: unquoted key
        let err_bad_json = ext_manager.trigger_pre_hook(
            &storage,
            "ADD",
            Some("json_bad"),
            Some("{name: \"Bob\"}"),
        );
        assert!(err_bad_json.is_err());
    }
}

