//! componentes de esquema compilados a partir dos XSDs (subconjunto usado
//! pelos leiautes fiscais)

mod carga;
pub mod dfa;
mod dom;
mod padrao;
pub mod simples;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

pub use carga::{ErroEsquema, carregar};
use dfa::Dfa;
use simples::TipoSimples;

pub const NS_XSD: &str = "http://www.w3.org/2001/XMLSchema";
pub const ANY_TYPE: IdTipo = 0;

pub type IdTipo = usize;
pub type IdElemento = usize;

type Chave = (Option<String>, String);

#[derive(Debug, Default)]
pub struct Esquema {
    tipos: Vec<Tipo>,
    elementos: Vec<DeclElemento>,
    elementos_globais: HashMap<Chave, IdElemento>,
    tipos_globais: HashMap<Chave, IdTipo>,
}

impl Esquema {
    pub fn tipo(&self, id: IdTipo) -> &Tipo {
        &self.tipos[id]
    }

    pub fn elemento(&self, id: IdElemento) -> &DeclElemento {
        &self.elementos[id]
    }

    pub fn elemento_global(&self, ns: Option<&str>, local: &str) -> Option<IdElemento> {
        self.elementos_globais
            .get(&(ns.map(str::to_string), local.to_string()))
            .copied()
    }

    pub fn tipo_global(&self, ns: Option<&str>, local: &str) -> Option<IdTipo> {
        self.tipos_globais
            .get(&(ns.map(str::to_string), local.to_string()))
            .copied()
    }

    /// derivação por restrição/extensão, com `anyType` acima de tudo
    pub fn deriva_de(&self, mut tipo: IdTipo, ancestral: IdTipo) -> bool {
        loop {
            if tipo == ancestral || ancestral == ANY_TYPE {
                return true;
            }
            let base = match &self.tipos[tipo] {
                Tipo::Simples(s) => s.base,
                Tipo::Complexo(c) => c.base,
            };
            match base {
                Some(b) if b != tipo => tipo = b,
                _ => return false,
            }
        }
    }
}

#[derive(Debug)]
pub enum Tipo {
    Simples(TipoSimples),
    Complexo(TipoComplexo),
}

impl Tipo {
    pub fn nome(&self) -> &str {
        match self {
            Self::Simples(s) => &s.nome,
            Self::Complexo(c) => &c.nome,
        }
    }
}

#[derive(Debug)]
pub struct TipoComplexo {
    pub nome: String,
    pub conteudo: Conteudo,
    pub atributos: Vec<UsoAtributo>,
    pub curinga_atributos: Option<Arc<Curinga>>,
    pub base: Option<IdTipo>,
    modelo: OnceLock<Dfa>,
}

impl TipoComplexo {
    fn novo(nome: String, conteudo: Conteudo) -> Self {
        Self {
            nome,
            conteudo,
            atributos: Vec::new(),
            curinga_atributos: None,
            base: None,
            modelo: OnceLock::new(),
        }
    }

    /// construído na primeira validação que precisar dele
    pub fn modelo(&self) -> Option<&Dfa> {
        match &self.conteudo {
            Conteudo::Elementos(p) => Some(self.modelo.get_or_init(|| Dfa::construir(Some(p)))),
            Conteudo::Misto(p) => Some(self.modelo.get_or_init(|| Dfa::construir(p.as_ref()))),
            Conteudo::Vazio | Conteudo::Simples(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Conteudo {
    Vazio,
    Simples(IdTipo),
    Elementos(Particula),
    Misto(Option<Particula>),
}

#[derive(Debug, Clone)]
pub struct Particula {
    pub termo: TermoParticula,
    pub min: u32,
    /// `None` é `unbounded`
    pub max: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum TermoParticula {
    Elemento(IdElemento),
    Curinga(Arc<Curinga>),
    Grupo(Compositor, Vec<Particula>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compositor {
    Sequencia,
    Escolha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Processamento {
    Pular,
    Lax,
    Estrito,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Namespaces {
    Qualquer,
    /// `##other`: nem o namespace alvo nem ausente
    Outro(Option<String>),
    Lista(Vec<Option<String>>),
}

#[derive(Debug, Clone)]
pub struct Curinga {
    pub namespaces: Namespaces,
    pub processamento: Processamento,
}

impl Curinga {
    pub fn aceita(&self, uri: Option<&str>) -> bool {
        match &self.namespaces {
            Namespaces::Qualquer => true,
            Namespaces::Outro(alvo) => uri.is_some() && uri != alvo.as_deref(),
            Namespaces::Lista(l) => l.iter().any(|n| n.as_deref() == uri),
        }
    }

    pub fn descricao(&self) -> String {
        let corpo = match &self.namespaces {
            Namespaces::Qualquer => "##any".to_string(),
            Namespaces::Outro(alvo) => format!("##other:\"{}\"", alvo.as_deref().unwrap_or("")),
            Namespaces::Lista(l) => l
                .iter()
                .map(|n| format!("\"{}\"", n.as_deref().unwrap_or("")))
                .collect::<Vec<_>>()
                .join(","),
        };
        format!("WC[{corpo}]")
    }
}

#[derive(Debug, Clone)]
pub struct UsoAtributo {
    pub nome: String,
    pub ns: Option<String>,
    pub tipo: IdTipo,
    pub obrigatorio: bool,
    pub fixo: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeclElemento {
    pub nome: String,
    pub ns: Option<String>,
    pub tipo: IdTipo,
    pub nillable: bool,
    pub fixo: Option<String>,
    pub padrao: Option<String>,
    pub unicos: Vec<Unico>,
}

impl DeclElemento {
    pub fn descricao(&self) -> String {
        match &self.ns {
            Some(ns) => format!("\"{ns}\":{}", self.nome),
            None => self.nome.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seletor {
    Filhos,
    Netos,
}

impl Seletor {
    pub fn profundidade(self) -> usize {
        match self {
            Self::Filhos => 1,
            Self::Netos => 2,
        }
    }
}

/// `xs:unique` com seletor `./*` ou `*/*` e campo `@atributo`
#[derive(Debug, Clone)]
pub struct Unico {
    pub nome: String,
    pub seletor: Seletor,
    pub atributo: String,
}
