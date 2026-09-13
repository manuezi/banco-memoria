use mlua::{Function, Lua, RegistryKey, Table};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

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
    ) -> Result<Option<String>, String> {
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
            .map_err(|e| format!("Failed to create context table: {e}"))?;

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
    ) -> Result<Option<String>, String> {
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
            .map_err(|e| format!("Failed to create context table: {e}"))?;

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

fn clean_lua_error(err: &mlua::Error) -> String {
    match err {
        mlua::Error::RuntimeError(msg) => {
            let first_line = msg.lines().next().unwrap_or(msg).trim();
            if let Some(rest) = first_line.strip_prefix("[string ")
                && let Some(end_quote) = rest.find("\"]:")
            {
                let after_quote = &rest[end_quote + 3..];
                if let Some(colon_space) = after_quote.find(": ") {
                    return after_quote[colon_space + 2..].to_string();
                }
            }
            first_line.to_string()
        }
        other => other.to_string(),
    }
}
