use std::fs;
use std::path::PathBuf;

use super::*;
use crate::json::ler_documento;

fn fixture(nome: &str) -> String {
    let caminho = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/xml/geracao")
        .join(nome);
    fs::read_to_string(&caminho).unwrap_or_else(|e| panic!("{}: {e}", caminho.display()))
}

fn roc(json: &str) -> Roc {
    match ler_documento::<Roc>(json.as_bytes()) {
        Ok(Some(r)) => r,
        outro => panic!("ROC inválido: {outro:?}"),
    }
}

fn mensagem(r: Resultado<String>) -> String {
    match r {
        Err(f) => f.to_string(),
        Ok(xml) => panic!("esperava falha, veio {xml}"),
    }
}

/// saídas do `XMLController.generate` da versão Java para o mesmo ROC
#[test]
fn gera_como_o_jaxb() {
    let entrada = roc(&fixture("roc.json"));
    for tipo in TipoDocumento::TODOS.into_iter().filter(|t| t.gera_xml()) {
        let esperado = fixture(&format!("{}.xml", tipo.mnemonico()));
        assert_eq!(gerar(&entrada, tipo).ok(), Some(esperado), "{tipo:?}");
    }
}

#[test]
fn grupo_vazio_usa_inicializadores_so_sem_builder() {
    let json = r#"{"objetos":[{"nObj":1,"tribCalc":{"IBSCBS":{"gIBSCBS":{"gIBSUF":{},"gCBS":{"gRed":{}}},"gIBSCBSMono":{}}}}],"total":{"tribCalc":{}}}"#;
    let xml = gerar(&roc(json), TipoDocumento::Nfce).unwrap_or_default();
    assert!(xml.contains("<gIBSUF>\n                        <pIBSUF>0</pIBSUF>\n                        <vIBSUF>0</vIBSUF>"));
    assert!(xml.contains("<gRed/>"));
    assert!(xml.contains("<vTotIBSMonoItem>0</vTotIBSMonoItem>"));
    assert!(xml.contains("<total/>"));
}

#[test]
fn falhas_na_ordem_do_java() {
    assert_eq!(mensagem(gerar(&roc("{}"), TipoDocumento::Nfe)), SEM_OBJETOS);
    assert_eq!(mensagem(gerar(&roc("{}"), TipoDocumento::Nf3e)), SEM_ITENS);
    assert_eq!(mensagem(gerar(&roc("{}"), TipoDocumento::Cte)), UM_ITEM);
    let sem_total = r#"{"objetos":[{"nObj":1,"tribCalc":{}}]}"#;
    assert_eq!(
        mensagem(gerar(&roc(sem_total), TipoDocumento::BpeTm)),
        SEM_TOTAL
    );
    let sem_numero = r#"{"objetos":[{"tribCalc":{}}],"total":{"tribCalc":{}}}"#;
    assert_eq!(
        mensagem(gerar(&roc(sem_numero), TipoDocumento::Nfe)),
        NOBJ_NULO
    );
    let nulo = r#"{"objetos":[null],"total":{"tribCalc":{}}}"#;
    assert_eq!(mensagem(gerar(&roc(nulo), TipoDocumento::Nfe)), OBJETO_NULO);
    assert_eq!(
        mensagem(gerar(&roc(nulo), TipoDocumento::Bpe)),
        PRIMEIRO_NULO
    );
    let sem_ibs = r#"{"objetos":[{"tribCalc":{}}]}"#;
    assert_eq!(
        mensagem(gerar(&roc(sem_ibs), TipoDocumento::Cte)),
        ORIGEM_NULA
    );
    assert_eq!(
        mensagem(gerar(&roc(sem_ibs), TipoDocumento::Nfse)),
        "Tipo de documento não suportado: NFSE"
    );
}

#[test]
fn erros_de_desserializacao_do_jackson() {
    let erro = |json: &str| match ler_documento::<Roc>(json.as_bytes()) {
        Err(e) => e.mensagem,
        Ok(r) => panic!("esperava erro: {r:?}"),
    };
    assert_eq!(
        erro(r#"{"objetos":[{"tribCalc":{"IBSCBS":{"gIBSCBS":{"gIBSUF":" "}}}}]}"#),
        "Cannot coerce empty String (\"\") to `br.gov.serpro.rtc.api.model.roc.IBSUFDomain` value (but could if coercion was enabled using `CoercionConfig`)"
    );
    assert_eq!(
        erro(r#"{"objetos":[{"calculoSimulado":"tRuE"}]}"#),
        "Cannot deserialize value of type `boolean` from String \"tRuE\": only \"true\"/\"True\"/\"TRUE\" or \"false\"/\"False\"/\"FALSE\" recognized"
    );
    assert_eq!(
        erro(r#"{"objetos":{}}"#),
        "Cannot deserialize value of type `java.util.ArrayList<br.gov.serpro.rtc.api.model.roc.ObjetoDomain>` from Object value (token `JsonToken.START_OBJECT`)"
    );
}
