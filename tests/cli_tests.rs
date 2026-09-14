use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

fn get_bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_banco-memoria")
}

fn run_pipeline(commands: &[&str]) -> Vec<String> {
    let mut child = Command::new(get_bin_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Falha ao iniciar o processo do banco-memoria");

    {
        let stdin = child.stdin.as_mut().expect("Falha ao abrir stdin");
        for cmd in commands {
            writeln!(stdin, "{cmd}").expect("Falha ao escrever no stdin");
        }
    }

    let output = child
        .wait_with_output()
        .expect("Falha ao aguardar encerramento do processo");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn test_cli_pipe_basic() {
    let output = run_pipeline(&[
        "ADD chave1 valor1",
        "GET chave1",
        "ADD chave1 novo_valor",
        "GET chave1",
        "EXIT",
    ]);

    assert_eq!(output, vec!["OK", "valor1", "OK", "novo_valor"]);
}

#[test]
fn test_cli_pipe_errors() {
    let output = run_pipeline(&[
        "GET chave_inexistente",
        "COMANDO_INVALIDO",
        "ADD",
        "ADD chave_sem_valor",
        "GET",
        "GET chave extra",
        "EXIT",
    ]);

    assert_eq!(
        output,
        vec![
            "ERRO: chave inexistente",
            "ERRO: comando desconhecido: COMANDO_INVALIDO",
            "ERRO: chave não informada",
            "ERRO: valor não informado",
            "ERRO: chave não informada",
            "ERRO: número excessivo de argumentos",
        ]
    );
}

#[test]
fn test_cli_pipe_casos_teste() {
    let content = fs::read_to_string("casos_teste.txt")
        .expect("Não foi possível ler o arquivo casos_teste.txt");

    let mut commands = Vec::new();
    let mut expected = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("# Esperado:") {
            expected.push(rest.trim().to_string());
        } else if trimmed.starts_with('#') {
            continue;
        } else {
            commands.push(trimmed);
        }
    }

    let output = run_pipeline(&commands);

    assert_eq!(
        output.len(),
        expected.len(),
        "Quantidade de saídas ({}) difere da esperada ({})",
        output.len(),
        expected.len()
    );

    for (i, (exp, act)) in expected.iter().zip(output.iter()).enumerate() {
        assert_eq!(
            exp, act,
            "Divergência na linha {i}: esperado '{exp}', obtido '{act}'"
        );
    }
}

#[test]
fn test_cli_pipe_casos_teste_json() {
    let Ok(content) = fs::read_to_string("casos_teste_json.txt") else {
        return;
    };

    let mut commands = Vec::new();
    let mut expected = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("# Esperado:") {
            expected.push(rest.trim().to_string());
        } else if trimmed.starts_with('#') {
            continue;
        } else {
            commands.push(trimmed);
        }
    }

    let output = run_pipeline(&commands);

    assert_eq!(
        output.len(),
        expected.len(),
        "Quantidade de saídas em json ({}) difere da esperada ({})",
        output.len(),
        expected.len()
    );

    for (i, (exp, act)) in expected.iter().zip(output.iter()).enumerate() {
        assert_eq!(
            exp, act,
            "Divergência em json na linha {i}: esperado '{exp}', obtido '{act}'"
        );
    }
}
