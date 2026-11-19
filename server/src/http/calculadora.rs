use hyper::StatusCode;
use rtc_core::modelo::entrada::Operacao;
use rtc_core::modelo::validacao::validar_operacao;
use rtc_core::servico::calculadora::aviso_dados_simulados;
use rtc_core::servico::observabilidade::{ErroObservabilidade, descrever_erro};

use super::App;
use super::erro::ErroHttp;
use super::pedido::Pedido;
use super::resposta::Resposta;
use crate::log;

const ASSINATURA_REGIME_GERAL: &str = "public org.springframework.http.ResponseEntity<br.gov.serpro.rtc.api.model.roc.ROCDomain> br.gov.serpro.rtc.api.controller.CalculadoraTributoController.calcularTributos(br.gov.serpro.rtc.api.model.input.OperacaoInput)";
const ASSINATURA_OBSERVABILIDADE: &str = "public org.springframework.http.ResponseEntity<br.gov.serpro.rtc.api.model.roc.ObservabilidadeROCDomain> br.gov.serpro.rtc.api.controller.ObservabilidadeController.calcularTributosObservabilidade(br.gov.serpro.rtc.api.model.input.OperacaoInput)";
const LOGGER_OBSERVABILIDADE: &str = "b.g.s.r.d.s.ObservabilidadeService";
const CABECALHO_AVISO: &str = "x-warning-dados-simulados";

/// `VersaoAplicacaoService.getHeaders` + aviso de dados simulados
fn com_versao(app: &App, r: Resposta, op: &Operacao) -> Result<Resposta, ErroHttp> {
    let r = app.cabecalhos_versao(r)?;
    Ok(match aviso_dados_simulados(op)? {
        Some(a) => r.com(CABECALHO_AVISO, &(a as u8).to_string()),
        None => r,
    })
}

pub fn regime_geral(app: &App, p: &Pedido<'_>) -> Result<Resposta, ErroHttp> {
    let op: Operacao = p.corpo_json(ASSINATURA_REGIME_GERAL)?;
    let violacoes = validar_operacao(&op);
    if !violacoes.is_empty() {
        return Err(ErroHttp::CamposInvalidos(violacoes));
    }
    let roc = app.com_servicos(|s| s.calcular_regime_geral(&op))?;
    com_versao(app, Resposta::serializar(StatusCode::OK, &roc)?, &op)
}

pub fn observabilidade(app: &App, p: &Pedido<'_>) -> Result<Resposta, ErroHttp> {
    let op: Operacao = p.corpo_json(ASSINATURA_OBSERVABILIDADE)?;
    let roc = app
        .com_servicos(|s| Ok(s.calcular_observabilidade(&op, p.base_url)))?
        .map_err(|e| match e {
            ErroObservabilidade::Erros(erros) => ErroHttp::Observabilidade(erros),
            ErroObservabilidade::Operacao { erros, causa } => {
                log::erro(
                    LOGGER_OBSERVABILIDADE,
                    &format!("Erro inesperado ao processar operação: {causa}"),
                );
                ErroHttp::Observabilidade(erros)
            }
            ErroObservabilidade::Falha(f) => ErroHttp::Falha(f),
        })?;
    com_versao(app, Resposta::serializar(StatusCode::OK, &roc)?, &op)
}

pub fn descricao_erro(_: &App, p: &Pedido<'_>) -> Result<Resposta, ErroHttp> {
    let codigo = p.obrigatorio("codigo")?;
    match descrever_erro(&codigo) {
        Some(d) => Ok(Resposta::serializar(StatusCode::OK, &d)?),
        None => Ok(Resposta::new(StatusCode::NOT_FOUND)),
    }
}
