use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum DatabaseError {
    NotFound,
    Syntax(String),
    Extension(String),
    Io(String),
}

impl DatabaseError {
    pub fn message(&self) -> &str {
        match self {
            DatabaseError::NotFound => "chave inexistente",
            DatabaseError::Syntax(msg) => msg.as_str(),
            DatabaseError::Extension(msg) => msg.as_str(),
            DatabaseError::Io(msg) => msg.as_str(),
        }
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERRO: {}", self.message())
    }
}

impl std::error::Error for DatabaseError {}

impl From<std::io::Error> for DatabaseError {
    fn from(err: std::io::Error) -> Self {
        DatabaseError::Io(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_format() {
        assert_eq!(DatabaseError::NotFound.to_string(), "ERRO: chave inexistente");
        assert_eq!(
            DatabaseError::Syntax("comando desconhecido: FOO".to_string()).to_string(),
            "ERRO: comando desconhecido: FOO"
        );
        assert_eq!(
            DatabaseError::Extension("CPF inválido".to_string()).to_string(),
            "ERRO: CPF inválido"
        );
        assert_eq!(
            DatabaseError::Io("falha de leitura".to_string()).to_string(),
            "ERRO: falha de leitura"
        );
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe quebrado");
        let db_err: DatabaseError = io_err.into();
        assert_eq!(db_err, DatabaseError::Io("pipe quebrado".to_string()));
    }
}
