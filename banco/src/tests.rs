use std::path::PathBuf;

use super::*;

fn diretorio_sql() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../flyway/sql")
}

struct BancoTemporario(PathBuf);

impl BancoTemporario {
    fn novo(nome: &str) -> Self {
        let p = std::env::temp_dir().join(format!("rtc-banco-{nome}-{}.db", std::process::id()));
        // resto de execução anterior com o mesmo pid
        if p.exists() {
            std::fs::remove_file(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        }
        Self(p)
    }
}

impl Drop for BancoTemporario {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.0) {
            eprintln!("não removeu {}: {e}", self.0.display());
        }
    }
}

fn historico(banco: &Path) -> R<Vec<(String, String, i32)>> {
    let c = Connection::open(banco)?;
    let mut stmt = c.prepare(
        "SELECT version, type, checksum FROM flyway_schema_history ORDER BY installed_rank",
    )?;
    let linhas = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    Ok(linhas.collect::<Result<_, _>>()?)
}

/// checksums gravados pelo Flyway 11 do projeto Java para os mesmos arquivos
#[test]
fn gera_como_o_flyway_e_e_idempotente() -> R<()> {
    let banco = BancoTemporario::novo("completo");
    let r = migrar(&diretorio_sql(), &banco.0)?;
    assert_eq!(r.aplicadas, 59);
    assert_eq!(r.versao_atual.as_deref(), Some("0059"));
    let h = historico(&banco.0)?;
    assert_eq!(h[0], ("0001".into(), "SQL_BASELINE".into(), -377_065_587));
    assert_eq!(h[1], ("0002".into(), "SQL".into(), 1_701_244_507));
    assert_eq!(h[58], ("0059".into(), "SQL".into(), -1_742_786_384));
    let de_novo = migrar(&diretorio_sql(), &banco.0)?;
    assert_eq!(de_novo.aplicadas, 0);
    Ok(())
}

#[test]
fn recusa_checksum_divergente() -> R<()> {
    let banco = BancoTemporario::novo("divergente");
    migrar(&diretorio_sql(), &banco.0)?;
    Connection::open(&banco.0)?.execute(
        "UPDATE flyway_schema_history SET checksum = 1 WHERE version = '0003'",
        [],
    )?;
    let erro = migrar(&diretorio_sql(), &banco.0);
    assert!(matches!(erro, Err(ErroBanco::Checksum { ref versao, .. }) if versao == "0003"));
    Ok(())
}
