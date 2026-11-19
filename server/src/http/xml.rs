use hyper::StatusCode;
use rtc_core::erro::{Classe, Falha};
use rtc_core::xml::geracao::{self, Roc};
use rtc_core::xml::{self, FalhaXml, TipoDocumento, TipoXml};

use super::App;
use super::erro::ErroHttp;
use super::pedido::Pedido;
use super::resposta::Resposta;
use crate::log;

const LOGGER_XML: &str = "b.g.s.r.d.s.x.XmlService";
const ASSINATURA_VALIDAR: &str = "public org.springframework.http.ResponseEntity<java.lang.Boolean> br.gov.serpro.rtc.api.controller.XMLController.validate(java.lang.String,br.gov.serpro.rtc.api.model.xml.enumeration.TipoDocumento,br.gov.serpro.rtc.api.model.xml.enumeration.TipoXml)";

const ASSINATURA_GERAR: &str = "public org.springframework.http.ResponseEntity<java.lang.String> br.gov.serpro.rtc.api.controller.XMLController.generate(br.gov.serpro.rtc.api.model.roc.ROCDomain,br.gov.serpro.rtc.api.model.xml.enumeration.TipoDocumento)";
const LOGGER_CONTROLLER: &str = "b.g.s.r.a.c.XMLController";

type Saida = Result<Resposta, ErroHttp>;

pub fn documentos_validacao(_: &App, _: &Pedido<'_>) -> Saida {
    Ok(Resposta::serializar(
        StatusCode::OK,
        &xml::documentos_validacao(),
    )?)
}

pub fn documentos_geracao(_: &App, _: &Pedido<'_>) -> Saida {
    Ok(Resposta::serializar(
        StatusCode::OK,
        &xml::documentos_geracao(),
    )?)
}

/// converter do Spring: em branco vira parâmetro ausente
fn parametro<T>(
    p: &Pedido<'_>,
    nome: &'static str,
    converter: fn(&str) -> Option<T>,
) -> Result<T, ErroHttp> {
    let Some(v) = p.parametro(nome).filter(|v| !v.trim().is_empty()) else {
        return Err(ErroHttp::ParametroAusente(nome));
    };
    converter(&v).ok_or(ErroHttp::TipoInvalido { nome, valor: v })
}

fn nome_constante(t: TipoDocumento) -> String {
    t.mnemonico().replace('-', "_").to_ascii_uppercase()
}

pub fn validar(_: &App, p: &Pedido<'_>) -> Saida {
    if p.corpo.is_empty() {
        return Err(ErroHttp::CorpoAusente(ASSINATURA_VALIDAR));
    }
    let tipo = parametro(p, "tipo", TipoDocumento::por_mnemonico)?;
    let subtipo = parametro(p, "subtipo", TipoXml::por_nome)?;
    let rotulo = format!(
        "TipoDocumento: {}, TipoXml: {}",
        nome_constante(tipo),
        if subtipo == TipoXml::Grupo {
            "GRUPO"
        } else {
            "NOTA"
        }
    );
    match xml::validar(&String::from_utf8_lossy(p.corpo), tipo, subtipo) {
        Ok(()) => {
            log::debug(
                LOGGER_XML,
                &format!("Validação de XML concluída com sucesso - {rotulo}"),
            );
            Ok(Resposta::json(StatusCode::OK, "true".into()))
        }
        Err(FalhaXml::Invalido(e)) => {
            log::erro(
                LOGGER_XML,
                &format!(
                    "Erro ao validar XML - {rotulo}\norg.xml.sax.SAXParseException; lineNumber: {}; columnNumber: {}; {}",
                    e.posicao.linha, e.posicao.coluna, e.mensagem
                ),
            );
            Err(Falha::new(Classe::ErroXml, format!("Erro na validação do XML: {e}")).into())
        }
        Err(FalhaXml::Esquema(m)) => Err(Falha::sistema(m).into()),
    }
}

/// O corpo é resolvido antes do parâmetro `tipo`
pub fn gerar(app: &App, p: &Pedido<'_>) -> Saida {
    let roc: Roc = p.corpo_json(ASSINATURA_GERAR)?;
    let tipo = parametro(p, "tipo", TipoDocumento::por_mnemonico)?;
    log::debug(LOGGER_CONTROLLER, "Gerando xml...");
    let xml = geracao::gerar(&roc, tipo)?;
    Ok(app.cabecalhos_versao(Resposta::xml(StatusCode::OK, xml))?)
}
