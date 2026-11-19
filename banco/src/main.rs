//! equivalente ao `./mvnw flyway:migrate -Dflyway.configFiles=./flyway/flyway-pro.conf`:
//! gera ou atualiza `./calculadora/db/calculadora-pro.db` a partir de `./flyway/sql`

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

const SQL_PADRAO: &str = "./flyway/sql";
const BANCO_PADRAO: &str = "./calculadora/db/calculadora-pro.db";
const USO: &str = "uso: rtc-gerar-banco [--sql DIRETORIO] [--banco ARQUIVO]";

struct Argumentos {
    sql: PathBuf,
    banco: PathBuf,
}

fn argumentos() -> Result<Argumentos, String> {
    let mut a = Argumentos {
        sql: PathBuf::from(SQL_PADRAO),
        banco: PathBuf::from(BANCO_PADRAO),
    };
    let mut lista = std::env::args().skip(1);
    while let Some(opcao) = lista.next() {
        let destino = match opcao.as_str() {
            "--sql" => &mut a.sql,
            "--banco" => &mut a.banco,
            _ => return Err(format!("opção desconhecida: {opcao}\n{USO}")),
        };
        *destino = lista
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| format!("{opcao} sem valor\n{USO}"))?;
    }
    Ok(a)
}

fn executar() -> Result<(), String> {
    let a = argumentos()?;
    if let Some(pai) = a.banco.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(pai).map_err(|e| format!("{}: {e}", pai.display()))?;
    }
    let inicio = Instant::now();
    let r = rtc_banco::migrar(&a.sql, &a.banco).map_err(|e| e.to_string())?;
    let versao = r.versao_atual.as_deref().unwrap_or("<< Empty Schema >>");
    let segundos = inicio.elapsed().as_secs_f64();
    if r.aplicadas == 0 {
        println!("Schema \"main\" is up to date. No migration necessary.");
    } else {
        println!(
            "Successfully applied {} migrations to schema \"main\", now at version v{versao} (execution time {segundos:.3}s)",
            r.aplicadas
        );
    }
    Ok(())
}

fn main() -> ExitCode {
    match executar() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ERROR: {e}");
            ExitCode::FAILURE
        }
    }
}
