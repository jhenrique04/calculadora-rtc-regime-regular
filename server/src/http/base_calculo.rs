use hyper::StatusCode;
use rtc_core::erro::Resultado;
use rtc_core::json::{Escrever, LerJson};
use rtc_core::modelo::base_calculo::{
    BaseUnificadaEntrada, BaseUnificadaNfseEntrada, CibsMercadorias, IsMercadorias,
    LayoutMercadorias, MercadoriasEntrada,
};
use rtc_core::modelo::validacao::{
    Violacao, validar_base_unificada, validar_base_unificada_nfse, validar_mercadorias,
};
use rtc_core::servico::base_calculo::{calcular_mercadorias, calcular_unificada};
use rtc_core::servico::base_calculo_nfse::calcular_unificada_nfse;

use super::App;
use super::erro::ErroHttp;
use super::pedido::Pedido;
use super::resposta::Resposta;

const ASSINATURA_UNIFICADA: &str = "public org.springframework.http.ResponseEntity<br.gov.serpro.rtc.api.model.output.basecalculo.BaseCalculoUnificadaOutput> br.gov.serpro.rtc.api.controller.BaseCalculoController.calcularBaseUnificada(br.gov.serpro.rtc.api.model.input.basecalculo.BaseCalculoUnificadaInput)";
const ASSINATURA_IS: &str = "public org.springframework.http.ResponseEntity<br.gov.serpro.rtc.api.model.output.basecalculo.BaseCalculoISMercadoriasModel> br.gov.serpro.rtc.api.controller.BaseCalculoController.calcularISMercadorias(br.gov.serpro.rtc.api.model.input.basecalculo.BaseCalculoISMercadoriasInput)";
const ASSINATURA_CIBS: &str = "public org.springframework.http.ResponseEntity<br.gov.serpro.rtc.api.model.output.basecalculo.BaseCalculoCibsModel> br.gov.serpro.rtc.api.controller.BaseCalculoController.calcularCibs(br.gov.serpro.rtc.api.model.input.basecalculo.BaseCalculoCibsInput)";
const ASSINATURA_NFSE: &str = "public org.springframework.http.ResponseEntity<br.gov.serpro.rtc.api.model.output.basecalculo.BaseCalculoUnificadaNfseOutput> br.gov.serpro.rtc.api.controller.BaseCalculoController.calcularBaseUnificadaNfse(br.gov.serpro.rtc.api.model.input.basecalculo.BaseCalculoUnificadaNfseInput)";

type Saida = Result<Resposta, ErroHttp>;

/// `@RequestBody @Valid`: lê, valida e só então chama o serviço
fn processar<E: LerJson + Default, S: Escrever>(
    p: &Pedido<'_>,
    assinatura: &'static str,
    validar: fn(&E) -> Vec<Violacao>,
    calcular: fn(&E) -> Resultado<S>,
) -> Saida {
    let e: E = p.corpo_json(assinatura)?;
    let violacoes = validar(&e);
    if !violacoes.is_empty() {
        return Err(ErroHttp::CamposInvalidos(violacoes));
    }
    Ok(Resposta::serializar(StatusCode::OK, &calcular(&e)?)?)
}

pub fn unificada(_: &App, p: &Pedido<'_>) -> Saida {
    processar::<BaseUnificadaEntrada, _>(
        p,
        ASSINATURA_UNIFICADA,
        validar_base_unificada,
        calcular_unificada,
    )
}

fn mercadorias<L: LayoutMercadorias>(p: &Pedido<'_>, assinatura: &'static str) -> Saida {
    processar::<MercadoriasEntrada<L>, _>(p, assinatura, validar_mercadorias, calcular_mercadorias)
}

pub fn is_mercadorias(_: &App, p: &Pedido<'_>) -> Saida {
    mercadorias::<IsMercadorias>(p, ASSINATURA_IS)
}

pub fn cibs_mercadorias(_: &App, p: &Pedido<'_>) -> Saida {
    mercadorias::<CibsMercadorias>(p, ASSINATURA_CIBS)
}

pub fn unificada_nfse(_: &App, p: &Pedido<'_>) -> Saida {
    processar::<BaseUnificadaNfseEntrada, _>(
        p,
        ASSINATURA_NFSE,
        validar_base_unificada_nfse,
        calcular_unificada_nfse,
    )
}
