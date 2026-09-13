use std::str::SplitWhitespace;

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

pub fn parse(input: &str) -> Result<Command, String> {
    let mut tokens = input.split_whitespace();
    let command_name = tokens.next().ok_or("nenhum comando fornecido".to_string())?;

    let command = match command_name.to_ascii_uppercase().as_str() {
        "ADD" => {
            let key = get_key(&mut tokens)?;
            let value = tokens.collect::<Vec<&str>>().join(" ");
            if value.is_empty() {
                return Err("valor não informado".to_string());
            }
            return Ok(Command::Add { key, value });
        }
        "GET" => {
            let key = get_key(&mut tokens)?;
            Command::Get { key }
        }
        "EXIT" => Command::Exit,
        other => return Err(format!("comando desconhecido: {other}")),
    };

    if tokens.next().is_some() {
        return Err("número excessivo de argumentos".to_string());
    }

    Ok(command)
}

fn get_key(tokens: &mut SplitWhitespace) -> Result<String, String> {
    let key = tokens.next().ok_or("chave não informada".to_string())?;
    Ok(key.to_string())
}

