//! validação de XML contra os XSDs dos documentos fiscais (`XmlService.validarXml`),
//! com scanner e validador próprios que reproduzem as mensagens do Xerces

mod caracteres;
pub mod esquema;
pub mod geracao;
mod leitor;
mod validador;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use esquema::Esquema;
pub use esquema::carregar;
pub use leitor::{ErroXml, Posicao};

mod embutidos {
    include!(concat!(env!("OUT_DIR"), "/xsd_embutidos.rs"));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TipoDocumento {
    Nfe,
    Nfce,
    Nfse,
    Cte,
    CteSimplificado,
    Bpe,
    BpeTm,
    Nf3e,
}

impl TipoDocumento {
    pub const TODOS: [Self; 8] = [
        Self::Nfe,
        Self::Nfce,
        Self::Nfse,
        Self::Cte,
        Self::CteSimplificado,
        Self::Bpe,
        Self::BpeTm,
        Self::Nf3e,
    ];

    pub fn mnemonico(self) -> &'static str {
        match self {
            Self::Nfe => "nfe",
            Self::Nfce => "nfce",
            Self::Nfse => "nfse",
            Self::Cte => "cte",
            Self::CteSimplificado => "cte-simplificado",
            Self::Bpe => "bpe",
            Self::BpeTm => "bpe-tm",
            Self::Nf3e => "nf3e",
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Self::Nfe => "NFe",
            Self::Nfce => "NFCe",
            Self::Nfse => "NFSe",
            Self::Cte => "CTe",
            Self::CteSimplificado => "CTe Simplificado",
            Self::Bpe => "BPe",
            Self::BpeTm => "BPeTM",
            Self::Nf3e => "NF3E",
        }
    }

    pub fn versao_nota_tecnica(self) -> &'static str {
        match self {
            Self::Nfe | Self::Nfce => "v1.36",
            Self::Nfse => "v1.01.03 - NT004",
            _ => "v1.14a",
        }
    }

    /// A NFS-e só tem validação, não geração
    pub fn gera_xml(self) -> bool {
        self != Self::Nfse
    }

    /// `fromMnemonico`: sem diferenciar maiúsculas
    pub fn por_mnemonico(s: &str) -> Option<Self> {
        Self::TODOS
            .into_iter()
            .find(|t| t.mnemonico().eq_ignore_ascii_case(s))
    }
}

#[derive(Debug, Clone)]
pub struct TipoDocumentoSaida {
    pub nome: Option<String>,
    pub mnemonico: Option<String>,
    pub versao_nota_tecnica: Option<String>,
}

crate::saida_json!(TipoDocumentoSaida {
    "nome" => nome: texto,
    "mnemonico" => mnemonico: texto,
    "versaoNotaTecnica" => versao_nota_tecnica: texto,
});

impl From<TipoDocumento> for TipoDocumentoSaida {
    fn from(t: TipoDocumento) -> Self {
        Self {
            nome: Some(t.nome().into()),
            mnemonico: Some(t.mnemonico().into()),
            versao_nota_tecnica: Some(t.versao_nota_tecnica().into()),
        }
    }
}

pub fn documentos_validacao() -> Vec<TipoDocumentoSaida> {
    TipoDocumento::TODOS.into_iter().map(Into::into).collect()
}

pub fn documentos_geracao() -> Vec<TipoDocumentoSaida> {
    TipoDocumento::TODOS
        .into_iter()
        .filter(|t| t.gera_xml())
        .map(Into::into)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TipoXml {
    Grupo,
    Nota,
}

impl TipoXml {
    /// `EnumUtils.getEnumIgnoreCase` pelo nome da constante
    pub fn por_nome(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "GRUPO" => Some(Self::Grupo),
            "NOTA" => Some(Self::Nota),
            _ => None,
        }
    }

    fn arquivo(self) -> &'static str {
        match self {
            Self::Grupo => "grupo.xsd",
            Self::Nota => "nota.xsd",
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum FalhaXml {
    /// `SAXParseException`: vira `ErroXmlException` (422)
    #[error("{0}")]
    Invalido(ErroXml),
    /// XSD embutido que não pôde ser carregado
    #[error("{0}")]
    Esquema(String),
}

fn arquivo_embutido(caminho: &str) -> Option<&'static [u8]> {
    embutidos::ARQUIVOS
        .iter()
        .find(|(nome, _)| *nome == caminho)
        .map(|(_, bytes)| *bytes)
}

type Cache = Mutex<HashMap<(TipoDocumento, TipoXml), Arc<Esquema>>>;

/// como o `@Cacheable` do `XmlUtil.getSchema`: compila uma vez por par
fn esquema(tipo: TipoDocumento, subtipo: TipoXml) -> Result<Arc<Esquema>, FalhaXml> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Cache::default);
    if let Some(e) = cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&(tipo, subtipo))
    {
        return Ok(Arc::clone(e));
    }
    let caminho = format!("{}/{}", tipo.mnemonico(), subtipo.arquivo());
    let compilado =
        Arc::new(carregar(&arquivo_embutido, &caminho).map_err(|e| FalhaXml::Esquema(e.0))?);
    cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert((tipo, subtipo), Arc::clone(&compilado));
    Ok(compilado)
}

pub fn validar(xml: &str, tipo: TipoDocumento, subtipo: TipoXml) -> Result<(), FalhaXml> {
    let esquema = esquema(tipo, subtipo)?;
    let mut v = validador::Validador::novo(&esquema);
    leitor::analisar(xml, &mut v).map_err(FalhaXml::Invalido)
}

#[cfg(test)]
mod tests;
