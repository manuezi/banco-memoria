use std::str::SplitWhitespace;

use crate::error::DatabaseError;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Add { key: String, value: String },
    Get { key: String },
    Exit,
}

impl Command {
    pub fn as_str(&self) -> &'static str {
        match self {
            Command::Add { .. } => "ADD",
            Command::Get { .. } => "GET",
            Command::Exit => "EXIT",
        }
    }

    pub fn params_as_tuple(&self) -> (Option<&str>, Option<&str>) {
        match self {
            Command::Add { key, value } => (Some(key.as_str()), Some(value.as_str())),
            Command::Get { key } => (Some(key.as_str()), None),
            Command::Exit => (None, None),
        }
    }
}

pub fn parse(input: &str) -> Result<Command, DatabaseError> {
    let mut tokens = input.split_whitespace();
    let command_name = tokens
        .next()
        .ok_or_else(|| DatabaseError::Syntax("nenhum comando fornecido".to_string()))?;

    let command = match command_name.to_ascii_uppercase().as_str() {
        "ADD" => {
            let key = get_key(&mut tokens)?;
            let value = tokens.collect::<Vec<&str>>().join(" ");
            if value.is_empty() {
                return Err(DatabaseError::Syntax("valor não informado".to_string()));
            }
            return Ok(Command::Add { key, value });
        }
        "GET" => {
            let key = get_key(&mut tokens)?;
            Command::Get { key }
        }
        "EXIT" => Command::Exit,
        other => return Err(DatabaseError::Syntax(format!("comando desconhecido: {other}"))),
    };

    if tokens.next().is_some() {
        return Err(DatabaseError::Syntax(
            "número excessivo de argumentos".to_string(),
        ));
    }

    Ok(command)
}

fn get_key(tokens: &mut SplitWhitespace) -> Result<String, DatabaseError> {
    let key = tokens
        .next()
        .ok_or_else(|| DatabaseError::Syntax("chave não informada".to_string()))?;
    Ok(key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_add() {
        match parse("ADD user_1 Alice").unwrap() {
            Command::Add { key, value } => {
                assert_eq!(key, "user_1");
                assert_eq!(value, "Alice");
            }
            _ => panic!("Expected Command::Add"),
        }

        match parse("add key multiple words in value").unwrap() {
            Command::Add { key, value } => {
                assert_eq!(key, "key");
                assert_eq!(value, "multiple words in value");
            }
            _ => panic!("Expected Command::Add"),
        }
    }

    #[test]
    fn test_parse_add_errors() {
        assert_eq!(
            parse("ADD").unwrap_err(),
            DatabaseError::Syntax("chave não informada".to_string())
        );
        assert_eq!(
            parse("ADD key").unwrap_err(),
            DatabaseError::Syntax("valor não informado".to_string())
        );
        assert_eq!(
            parse("ADD key    ").unwrap_err(),
            DatabaseError::Syntax("valor não informado".to_string())
        );
    }

    #[test]
    fn test_parse_get() {
        match parse("GET user_1").unwrap() {
            Command::Get { key } => assert_eq!(key, "user_1"),
            _ => panic!("Expected Command::Get"),
        }

        match parse("get user_1").unwrap() {
            Command::Get { key } => assert_eq!(key, "user_1"),
            _ => panic!("Expected Command::Get"),
        }
    }

    #[test]
    fn test_parse_get_errors() {
        assert_eq!(
            parse("GET").unwrap_err(),
            DatabaseError::Syntax("chave não informada".to_string())
        );
        assert_eq!(
            parse("GET key extra").unwrap_err(),
            DatabaseError::Syntax("número excessivo de argumentos".to_string())
        );
    }

    #[test]
    fn test_parse_exit() {
        assert!(matches!(parse("EXIT").unwrap(), Command::Exit));
        assert!(matches!(parse("exit").unwrap(), Command::Exit));
        assert_eq!(
            parse("EXIT extra").unwrap_err(),
            DatabaseError::Syntax("número excessivo de argumentos".to_string())
        );
    }

    #[test]
    fn test_parse_empty_and_unknown() {
        assert_eq!(
            parse("").unwrap_err(),
            DatabaseError::Syntax("nenhum comando fornecido".to_string())
        );
        assert_eq!(
            parse("   \t   ").unwrap_err(),
            DatabaseError::Syntax("nenhum comando fornecido".to_string())
        );
        assert_eq!(
            parse("INVALID_CMD key").unwrap_err(),
            DatabaseError::Syntax("comando desconhecido: INVALID_CMD".to_string())
        );
    }

    #[test]
    fn test_command_helpers() {
        let add = Command::Add {
            key: "k".to_string(),
            value: "v".to_string(),
        };
        assert_eq!(add.as_str(), "ADD");
        assert_eq!(add.params_as_tuple(), (Some("k"), Some("v")));

        let get = Command::Get {
            key: "k".to_string(),
        };
        assert_eq!(get.as_str(), "GET");
        assert_eq!(get.params_as_tuple(), (Some("k"), None));

        let exit = Command::Exit;
        assert_eq!(exit.as_str(), "EXIT");
        assert_eq!(exit.params_as_tuple(), (None, None));
    }
}
