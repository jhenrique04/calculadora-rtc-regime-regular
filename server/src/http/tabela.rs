//! mapeamentos dos controllers (`@RequestMapping` relativo ao context-path)

use hyper::Method;

use super::midia::{JSON, XML};
use super::rotas::{Manipulador, Rota};
use super::{
    base_calculo as bc, calculadora, dados_abertos as da, nfse, pedagio, springdoc, versao, xml,
};

fn post_json(padrao: &'static str, manipulador: Manipulador) -> Rota {
    Rota {
        padrao,
        metodo: Method::POST,
        consome: Some(JSON),
        produz: JSON,
        manipulador,
    }
}

fn post_xml(padrao: &'static str, manipulador: Manipulador) -> Rota {
    Rota {
        padrao,
        metodo: Method::POST,
        consome: Some(XML),
        produz: JSON,
        manipulador,
    }
}

/// mapeamento sem `produces` (os redirecionamentos do springdoc)
const QUALQUER: &str = "*/*";

fn get(padrao: &'static str, produz: &'static str, manipulador: Manipulador) -> Rota {
    Rota {
        padrao,
        metodo: Method::GET,
        consome: None,
        produz,
        manipulador,
    }
}

fn get_json(padrao: &'static str, manipulador: Manipulador) -> Rota {
    get(padrao, JSON, manipulador)
}

pub fn rotas() -> Vec<Rota> {
    vec![
        post_json("/calculadora/regime-geral", calculadora::regime_geral),
        post_json(
            "/calculadora/observabilidade/regime-geral",
            calculadora::observabilidade,
        ),
        get_json(
            "/calculadora/observabilidade/erros",
            calculadora::descricao_erro,
        ),
        post_json("/calculadora/pedagio", pedagio::calcular),
        post_json(
            "/calculadora/base-calculo/base-calculo-unificada",
            bc::unificada,
        ),
        post_json(
            "/calculadora/base-calculo/is-mercadorias",
            bc::is_mercadorias,
        ),
        post_json(
            "/calculadora/base-calculo/cbs-ibs-mercadorias",
            bc::cibs_mercadorias,
        ),
        post_json(
            "/calculadora/base-calculo/base-calculo-unificada-nfse",
            bc::unificada_nfse,
        ),
        get_json("/versao/status", versao::status),
        post_xml("/calculadora/xml/validate", xml::validar),
        get_json("/calculadora/xml/validate", xml::documentos_validacao),
        get_json("/calculadora/xml/generate", xml::documentos_geracao),
        get("/", QUALQUER, springdoc::raiz),
        get("/swagger-ui.html", QUALQUER, springdoc::swagger_ui_html),
        get("/api-docs", JSON, springdoc::api_docs),
        get(
            "/api-docs.yaml",
            springdoc::OPENAPI_YAML,
            springdoc::api_docs_yaml,
        ),
        get("/api-docs/swagger-config", JSON, springdoc::configuracao),
        Rota {
            padrao: "/calculadora/xml/generate",
            metodo: Method::POST,
            consome: Some(JSON),
            produz: XML,
            manipulador: xml::gerar,
        },
        get_json("/calculadora/download/url", versao::download_url),
        post_json("/calculadora/nfse/base-calculo", nfse::base_calculo),
        post_json(
            "/calculadora/nfse/indicador-operacao/validate",
            nfse::validar_indicador,
        ),
        get_json("/calculadora/nfse/local-operacao", nfse::local_operacao),
        get_json("/calculadora/nfse/indicador-operacao", nfse::indicadores),
        get_json(
            "/calculadora/nfse/locais-fornecimento",
            nfse::locais_fornecimento,
        ),
        get_json(
            "/calculadora/nfse/locais-incidencia",
            nfse::locais_incidencia,
        ),
        get_json(
            "/calculadora/nfse/situacoes-classificacoes-tributarias",
            nfse::situacoes_classificacoes,
        ),
        get_json("/calculadora/dados-abertos/ufs", da::ufs),
        get_json("/calculadora/dados-abertos/ufs/municipios", da::municipios),
        get_json(
            "/calculadora/dados-abertos/situacoes-tributarias/cbs-ibs",
            da::csts_cbs_ibs,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/{idSituacaoTributaria}",
            da::classificacoes_por_situacao,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/imposto-seletivo/{cst}",
            da::classificacoes_cst_is,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/cbs-ibs/{cst}",
            da::classificacoes_cst_cbs_ibs,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/cbs-ibs/class-trib/{cClassTrib}",
            da::classificacao_cbs_ibs,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/is/class-trib/{cClassTrib}",
            da::classificacao_is,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/cbs-ibs",
            da::classificacoes_cbs_ibs,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/cbs-ibs/por-atores",
            da::classificacoes_por_atores,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/imposto-seletivo",
            da::classificacoes_is,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/nbs",
            da::classificacoes_por_nbs,
        ),
        get_json(
            "/calculadora/dados-abertos/situacoes-tributarias/imposto-seletivo",
            da::situacoes_is,
        ),
        get_json("/calculadora/dados-abertos/ncm", da::ncm),
        get_json("/calculadora/dados-abertos/ncm/vigentes", da::ncms_vigentes),
        get_json("/calculadora/dados-abertos/nbs", da::nbs),
        get_json("/calculadora/dados-abertos/nbs/lista", da::lista_nbs),
        get_json(
            "/calculadora/dados-abertos/nbs-aplicaveis",
            da::nbs_aplicaveis,
        ),
        get_json(
            "/calculadora/dados-abertos/fundamentacoes-legais",
            da::fundamentacoes,
        ),
        get_json(
            "/calculadora/dados-abertos/aliquota-uniao",
            da::aliquota_uniao,
        ),
        get_json("/calculadora/dados-abertos/aliquota-uf", da::aliquota_uf),
        get_json(
            "/calculadora/dados-abertos/aliquota-municipio",
            da::aliquota_municipio,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/cbs-ibs/{siglaDfe}/{cClassTrib}",
            da::validade_dfe,
        ),
        get_json(
            "/calculadora/dados-abertos/situacoes-tributarias/is",
            da::csts_is,
        ),
        get_json("/calculadora/dados-abertos/dfe/grupos", da::grupos_dfe),
        get_json("/calculadora/dados-abertos/versao", da::versao),
        get_json(
            "/calculadora/dados-abertos/redutores-compra-governamental",
            da::redutores,
        ),
        get_json(
            "/calculadora/dados-abertos/transferencias-cbs",
            da::transferencias_cbs,
        ),
        get_json(
            "/calculadora/dados-abertos/transferencias-ibs",
            da::transferencias_ibs,
        ),
        get_json(
            "/calculadora/dados-abertos/nomenclatura/{siglaDfe}/{cClassTrib}",
            da::nomenclatura,
        ),
        get_json("/calculadora/dados-abertos/ator/grupos", da::grupos_ator),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/nbs-aplicavel",
            da::nbs_aplicavel,
        ),
        get_json(
            "/calculadora/dados-abertos/classificacoes-tributarias/ncm-aplicavel",
            da::ncm_aplicavel,
        ),
    ]
}
