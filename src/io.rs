use std::io::{self, stdin, IsTerminal, Write};

fn is_interactive() -> bool {
    stdin().is_terminal()
}

pub fn read() -> io::Result<Option<String>> {
    if is_interactive() {
        print!("> ");
        io::stdout().flush()?
    }

    let mut buffer = String::new();
    match io::stdin().read_line(&mut buffer)? {
        0 => Ok(None),
        _ => Ok(Some(buffer.trim().to_string())),
    }
}

pub fn print_success() {
    println!("OK");
}

pub fn print_value(value: &str) {
    println!("{}", value);
}

pub fn print_error(message: &str) {
    let msg = message.trim();
    if msg.starts_with("ERRO:") {
        println!("{}", msg);
    } else {
        println!("ERRO: {}", msg);
    }
}
