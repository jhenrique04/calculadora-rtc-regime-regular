//! árvore mínima de um documento XSD, com os namespaces em escopo de cada nó
//! (necessários para resolver QNames em `type`, `base` e `ref`)

use std::collections::HashMap;
use std::rc::Rc;

use super::NS_XSD;
use crate::xml::leitor::{Elemento, ErroXml, Manipulador, NS_XMLNS, Namespaces, Posicao, analisar};

#[derive(Debug)]
pub struct No {
    pub local: String,
    pub uri: Option<String>,
    atributos: Vec<(String, String)>,
    pub filhos: Vec<No>,
    pub escopo: Rc<HashMap<String, String>>,
}

impl No {
    /// só atributos sem prefixo, que é o que os XSDs usam
    pub fn atributo(&self, nome: &str) -> Option<&str> {
        self.atributos
            .iter()
            .find(|(n, _)| n == nome)
            .map(|(_, v)| v.as_str())
    }

    pub fn nome(&self) -> &str {
        self.atributo("name").unwrap_or("")
    }

    /// filhos do namespace XSD, sem `annotation`
    pub fn filhos_xsd(&self) -> impl Iterator<Item = &No> {
        self.filhos
            .iter()
            .filter(|f| f.uri.as_deref() == Some(NS_XSD) && f.local != "annotation")
    }

    pub fn filho(&self, local: &str) -> Option<&No> {
        self.filhos_xsd().find(|f| f.local == local)
    }
}

#[derive(Default)]
struct Construtor {
    pilha: Vec<No>,
    raiz: Option<No>,
}

impl Manipulador for Construtor {
    fn inicio(&mut self, e: &Elemento, ns: &Namespaces, _: Posicao) -> Result<(), ErroXml> {
        let declara = e
            .atributos
            .iter()
            .any(|a| a.nome.uri.as_deref() == Some(NS_XMLNS));
        let escopo = match self.pilha.last() {
            Some(pai) if !declara => Rc::clone(&pai.escopo),
            _ => Rc::new(ns.visiveis()),
        };
        let atributos = e
            .atributos
            .iter()
            .filter(|a| a.nome.prefixo.is_none() && a.nome.uri.is_none())
            .map(|a| (a.nome.local.clone(), a.valor.clone()))
            .collect();
        self.pilha.push(No {
            local: e.nome.local.clone(),
            uri: e.nome.uri.clone(),
            atributos,
            filhos: Vec::new(),
            escopo,
        });
        Ok(())
    }

    fn texto(&mut self, _: &str) {}

    fn fim(&mut self, _: Posicao) -> Result<(), ErroXml> {
        let Some(no) = self.pilha.pop() else {
            return Ok(());
        };
        match self.pilha.last_mut() {
            Some(pai) => pai.filhos.push(no),
            None => self.raiz = Some(no),
        }
        Ok(())
    }
}

pub fn ler(texto: &str) -> Result<No, ErroXml> {
    let mut c = Construtor::default();
    analisar(texto, &mut c)?;
    c.raiz.ok_or_else(|| ErroXml {
        posicao: Posicao {
            linha: 1,
            coluna: 1,
        },
        mensagem: "documento sem elemento raiz".into(),
    })
}
