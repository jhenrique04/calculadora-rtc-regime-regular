//! `flyway:migrate` do projeto Java (Flyway 11 sobre SQLite): migração de
//! baseline, migrações versionadas em ordem, callbacks `beforeMigrate` e
//! `afterMigrate` e a tabela `flyway_schema_history`

mod descoberta;

use std::path::{Path, PathBuf};
use std::time::Instant;

use rusqlite::{Connection, OptionalExtension, params};

pub use descoberta::{Migracao, Tipo};

const HISTORICO_DDL: &str = r#"CREATE TABLE "flyway_schema_history" (
    "installed_rank" INT NOT NULL PRIMARY KEY,
    "version" VARCHAR(50),
    "description" VARCHAR(200) NOT NULL,
    "type" VARCHAR(20) NOT NULL,
    "script" VARCHAR(1000) NOT NULL,
    "checksum" INT,
    "installed_by" VARCHAR(100) NOT NULL,
    "installed_on" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f','now')),
    "execution_time" INT NOT NULL,
    "success" BOOLEAN NOT NULL
)"#;
const HISTORICO_INDICE: &str =
    r#"CREATE INDEX "flyway_schema_history_s_idx" ON "flyway_schema_history" ("success")"#;
// flyway.user vazio no flyway-pro.conf
const INSTALADO_POR: &str = "";

#[derive(Debug, thiserror::Error)]
pub enum ErroBanco {
    #[error("falha ao ler {caminho}: {fonte}")]
    Leitura {
        caminho: PathBuf,
        fonte: std::io::Error,
    },
    #[error("nome de migração inválido: {0}")]
    Nome(String),
    #[error("Found more than one migration with version {0}")]
    VersaoDuplicada(String),
    #[error(
        "Validate failed: Migrations have failed validation\nMigration checksum mismatch for migration version {versao}\n-> Applied to database : {aplicado}\n-> Resolved locally    : {local}"
    )]
    Checksum {
        versao: String,
        aplicado: i32,
        local: i32,
    },
    #[error(
        "Validate failed: Migrations have failed validation\nDetected applied migration not resolved locally: {0}."
    )]
    NaoResolvida(String),
    #[error(
        "Validate failed: Migrations have failed validation\nDetected resolved migration not applied to database: {0}."
    )]
    ForaDeOrdem(String),
    #[error("Migration {script} failed\n{fonte}")]
    Script {
        script: String,
        fonte: rusqlite::Error,
    },
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub type R<T> = Result<T, ErroBanco>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relatorio {
    pub aplicadas: usize,
    pub versao_atual: Option<String>,
}

struct Aplicada {
    versao: String,
    checksum: Option<i32>,
}

/// O Flyway não abre transação em script com `PRAGMA foreign_keys`: dentro dela
/// o SQLite ignora o pragma
fn transacional(sql: &str) -> bool {
    !sql.lines().any(|l| {
        l.trim_start()
            .to_ascii_lowercase()
            .starts_with("pragma foreign_keys")
    })
}

fn executar(conexao: &Connection, script: &str, sql: &str) -> R<()> {
    let falha = |fonte| ErroBanco::Script {
        script: script.to_string(),
        fonte,
    };
    if !transacional(sql) {
        return conexao.execute_batch(sql).map_err(falha);
    }
    // o drop da transação desfaz tudo se algum comando falhar
    let tx = conexao.unchecked_transaction()?;
    tx.execute_batch(sql).map_err(falha)?;
    Ok(tx.commit()?)
}

fn historico_existe(conexao: &Connection) -> R<bool> {
    let existe = conexao
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'flyway_schema_history'",
            [],
            |_| Ok(()),
        )
        .optional()?;
    Ok(existe.is_some())
}

fn aplicadas(conexao: &Connection) -> R<Vec<Aplicada>> {
    let mut stmt = conexao.prepare(
        "SELECT version, checksum FROM flyway_schema_history \
         WHERE success = 1 AND version IS NOT NULL ORDER BY installed_rank",
    )?;
    let linhas = stmt.query_map([], |r| {
        Ok(Aplicada {
            versao: r.get(0)?,
            checksum: r.get(1)?,
        })
    })?;
    Ok(linhas.collect::<Result<_, _>>()?)
}

/// `validateOnMigrate`: o que já foi aplicado tem de existir localmente, igual
fn validar(locais: &[Migracao], ja: &[Aplicada]) -> R<()> {
    for a in ja {
        let Some(m) = locais.iter().find(|m| m.versao == a.versao) else {
            return Err(ErroBanco::NaoResolvida(a.versao.clone()));
        };
        if a.checksum != Some(m.checksum) {
            return Err(ErroBanco::Checksum {
                versao: a.versao.clone(),
                aplicado: a.checksum.unwrap_or_default(),
                local: m.checksum,
            });
        }
    }
    Ok(())
}

/// banco vazio: a baseline mais recente e as versionadas acima dela. Banco
/// existente: as versionadas ainda não aplicadas, sem aceitar fora de ordem
fn pendentes<'m>(locais: &'m [Migracao], ja: &[Aplicada]) -> R<Vec<&'m Migracao>> {
    let versionadas = locais.iter().filter(|m| m.tipo == Tipo::Versionada);
    if ja.is_empty() {
        let baseline = locais.iter().rfind(|m| m.tipo == Tipo::Baseline);
        let acima = |m: &&Migracao| baseline.is_none_or(|b| m.numero > b.numero);
        return Ok(baseline
            .into_iter()
            .chain(versionadas.filter(acima))
            .collect());
    }
    let atual = locais
        .iter()
        .filter(|m| ja.iter().any(|a| a.versao == m.versao))
        .map(|m| &m.numero)
        .max();
    let mut saida = Vec::new();
    for m in versionadas.filter(|m| !ja.iter().any(|a| a.versao == m.versao)) {
        if atual.is_some_and(|v| &m.numero < v) {
            return Err(ErroBanco::ForaDeOrdem(m.versao.clone()));
        }
        saida.push(m);
    }
    Ok(saida)
}

fn proximo_rank(conexao: &Connection) -> R<i64> {
    Ok(conexao.query_row(
        "SELECT COALESCE(MAX(installed_rank), 0) + 1 FROM flyway_schema_history",
        [],
        |r| r.get(0),
    )?)
}

fn registrar(conexao: &Connection, rank: i64, m: &Migracao, duracao_ms: i64) -> R<()> {
    conexao.execute(
        "INSERT INTO flyway_schema_history (installed_rank, version, description, type, script, \
         checksum, installed_by, execution_time, success) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1)",
        params![
            rank,
            m.versao,
            m.descricao,
            m.tipo.nome(),
            m.script,
            m.checksum,
            INSTALADO_POR,
            duracao_ms
        ],
    )?;
    Ok(())
}

fn aplicar(conexao: &Connection, m: &Migracao) -> R<()> {
    let inicio = Instant::now();
    executar(conexao, &m.script, &m.conteudo)?;
    let duracao = i64::try_from(inicio.elapsed().as_millis()).unwrap_or(i64::MAX);
    let rank = proximo_rank(conexao)?;
    registrar(conexao, rank, m, duracao)
}

fn callback(conexao: &Connection, c: Option<&descoberta::Callback>) -> R<()> {
    match c {
        Some(c) => executar(conexao, &c.script, &c.conteudo),
        None => Ok(()),
    }
}

/// abre (criando se preciso) o banco e aplica o que estiver pendente
pub fn migrar(diretorio_sql: &Path, banco: &Path) -> R<Relatorio> {
    let scripts = descoberta::descobrir(diretorio_sql)?;
    // mesma conexão do flyway-pro.conf: foreign_keys=ON na abertura
    let conexao = Connection::open(banco)?;
    conexao.pragma_update(None, "foreign_keys", "ON")?;
    // o Flyway cria o histórico antes de rodar o beforeMigrate
    if !historico_existe(&conexao)? {
        conexao.execute_batch(&format!("{HISTORICO_DDL};\n{HISTORICO_INDICE};"))?;
    }
    callback(&conexao, scripts.antes.as_ref())?;
    let ja = aplicadas(&conexao)?;
    validar(&scripts.migracoes, &ja)?;
    let pendentes = pendentes(&scripts.migracoes, &ja)?;
    for m in &pendentes {
        aplicar(&conexao, m)?;
    }
    callback(&conexao, scripts.depois.as_ref())?;
    let versao_atual = aplicadas(&conexao)?.pop().map(|a| a.versao);
    Ok(Relatorio {
        aplicadas: pendentes.len(),
        versao_atual,
    })
}

#[cfg(test)]
mod tests;
