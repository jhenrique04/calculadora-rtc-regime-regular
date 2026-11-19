//! configuração do perfil `offline` (application-offline.yml) sobrescrevível
//! pelas mesmas variáveis de ambiente do Spring Boot (relaxed binding)

use std::path::PathBuf;

use crate::log::Nivel;

const PORTA_PADRAO: u16 = 8080;
const PORTA_GESTAO_PADRAO: u16 = 9101;
const ARQUIVO_DB_PADRAO: &str = "calculadora-pro.db";
const DIRETORIO_DB: &str = "./calculadora/db";
const SUFIXO_DB_PRODUCAO: &str = "-pro.db";
const PERFIS_ATIVOS: &str = "offline";
const URL_VERSAO_PADRAO: &str = "https://piloto-cbs.tributos.gov.br/servico/calculadora-consumo/api/calculadora/dados-abertos/versao";
const PREFIXO_PLATAFORMAS: &str = "APPLICATION_DOWNLOAD_PLATFORMS_";

#[derive(Debug, Clone)]
pub struct Config {
    pub porta: u16,
    pub porta_gestao: u16,
    pub arquivo_db: String,
    pub caminho_db: PathBuf,
    pub nivel_log: Nivel,
    pub url_versao: String,
    /// `application.download.platforms` (nome, url)
    pub plataformas_download: Vec<(String, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum ErroConfig {
    #[error("Failed to bind properties under '{0}': valor inválido '{1}'")]
    Invalido(&'static str, String),
    #[error("Nome do arquivo do banco de dados não pode ser nulo, vazio ou conter espaços.")]
    NomeDb,
    #[error(
        "Nome do banco de dados '{0}' inválido para profiles '{PERFIS_ATIVOS}'. Deve terminar com '{SUFIXO_DB_PRODUCAO}'"
    )]
    SufixoDb(String),
}

fn ler_porta(var: &'static str, chave: &'static str, padrao: u16) -> Result<u16, ErroConfig> {
    match std::env::var(var) {
        Ok(v) => v.trim().parse().map_err(|_| ErroConfig::Invalido(chave, v)),
        Err(_) => Ok(padrao),
    }
}

/// chaves de mapa vindas do ambiente seguem o relaxed binding: `MAC_ARM` vira `mac.arm`
fn plataformas_do_ambiente() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = std::env::vars()
        .filter_map(|(k, url)| {
            let nome = k.strip_prefix(PREFIXO_PLATAFORMAS)?;
            Some((nome.to_lowercase().replace('_', "."), url))
        })
        .filter(|(nome, _)| !nome.is_empty())
        .collect();
    v.sort();
    v
}

impl Config {
    pub fn carregar() -> Result<Self, ErroConfig> {
        let porta = ler_porta("SERVER_PORT", "server.port", PORTA_PADRAO)?;
        let porta_gestao = ler_porta(
            "MANAGEMENT_SERVER_PORT",
            "management.server.port",
            PORTA_GESTAO_PADRAO,
        )?;
        let arquivo_db = std::env::var("APPLICATION_DB_FILENAME")
            .unwrap_or_else(|_| ARQUIVO_DB_PADRAO.to_string());
        // databaseValidator
        if arquivo_db.trim().is_empty() || arquivo_db.contains(' ') {
            return Err(ErroConfig::NomeDb);
        }
        if !arquivo_db.ends_with(SUFIXO_DB_PRODUCAO) {
            return Err(ErroConfig::SufixoDb(arquivo_db));
        }
        let nivel_log = match std::env::var("LOGGING_LEVEL_ROOT") {
            Ok(v) => Nivel::parse(&v).ok_or(ErroConfig::Invalido("logging.level.root", v))?,
            Err(_) => Nivel::Info,
        };
        Ok(Self {
            porta,
            porta_gestao,
            caminho_db: PathBuf::from(DIRETORIO_DB).join(&arquivo_db),
            arquivo_db,
            nivel_log,
            url_versao: std::env::var("VERSAO_OFFLINE_URL")
                .unwrap_or_else(|_| URL_VERSAO_PADRAO.to_string()),
            plataformas_download: plataformas_do_ambiente(),
        })
    }
}
