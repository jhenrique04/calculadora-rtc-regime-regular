use hyper::StatusCode;
use rtc_core::modelo::nfse::{NfseBaseCalculoEntrada, NfseIndicadorOperacaoEntrada};
use rtc_core::modelo::validacao::{validar_nfse_base_calculo, validar_nfse_indicador};
use rtc_core::servico::nfse::{TipoLocal, calcular_base_calculo, locais};

use super::App;
use super::erro::ErroHttp;
use super::pedido::{Pedido, exigir_digitos};
use super::resposta::Resposta;

const ASSINATURA_BASE_CALCULO: &str = "public org.springframework.http.ResponseEntity<br.gov.serpro.rtc.api.model.output.nfse.NfseBaseCalculoOutput> br.gov.serpro.rtc.api.controller.NfseController.calcularBaseCalculo(br.gov.serpro.rtc.api.model.input.nfse.NfseBaseCalculoInput)";
const ASSINATURA_VALIDAR: &str = "public org.springframework.http.ResponseEntity<br.gov.serpro.rtc.api.model.output.nfse.NfseValidacaoIndicadorOperacaoOutput> br.gov.serpro.rtc.api.controller.NfseController.validarIndicadorOperacao(br.gov.serpro.rtc.api.model.input.nfse.NfseValidacaoIndicadorOperacaoInput)";

const TAM_C_IND_OP: usize = 6;
const TAM_NBS: usize = 9;

type Saida = Result<Resposta, ErroHttp>;

fn sem_violacoes(v: Vec<rtc_core::modelo::validacao::Violacao>) -> Result<(), ErroHttp> {
    if v.is_empty() {
        Ok(())
    } else {
        Err(ErroHttp::CamposInvalidos(v))
    }
}

pub fn base_calculo(_: &App, p: &Pedido<'_>) -> Saida {
    let e: NfseBaseCalculoEntrada = p.corpo_json(ASSINATURA_BASE_CALCULO)?;
    sem_violacoes(validar_nfse_base_calculo(&e))?;
    Ok(Resposta::serializar(
        StatusCode::OK,
        &calcular_base_calculo(&e)?,
    )?)
}

pub fn validar_indicador(app: &App, p: &Pedido<'_>) -> Saida {
    let e: NfseIndicadorOperacaoEntrada = p.corpo_json(ASSINATURA_VALIDAR)?;
    sem_violacoes(validar_nfse_indicador(&e))?;
    let v = app.com_servicos(|s| s.validar_indicador_operacao(&e))?;
    Ok(Resposta::serializar(StatusCode::OK, &v)?)
}

pub fn local_operacao(app: &App, p: &Pedido<'_>) -> Saida {
    let c_ind_op = p.obrigatorio("cIndOp")?;
    let dia = p.data_obrigatoria("dataOcorrenciaFatoGerador")?;
    exigir_digitos(Some(&c_ind_op), TAM_C_IND_OP)?;
    let v = app.com_servicos(|s| s.local_operacao(&c_ind_op, dia))?;
    Ok(Resposta::serializar(StatusCode::OK, &v)?)
}

pub fn indicadores(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("dataOcorrenciaFatoGerador")?;
    let nbs = p.parametro("nbs");
    exigir_digitos(nbs.as_deref(), TAM_NBS)?;
    let v = app.com_servicos(|s| s.indicadores_operacao(dia, nbs.as_deref()))?;
    Ok(Resposta::serializar(StatusCode::OK, &v)?)
}

fn locais_de(p: &Pedido<'_>, tipo: TipoLocal) -> Saida {
    let codigo = p.inteiro("codigo")?;
    Ok(Resposta::serializar(
        StatusCode::OK,
        &locais(tipo, codigo)?,
    )?)
}

pub fn locais_fornecimento(_: &App, p: &Pedido<'_>) -> Saida {
    locais_de(p, TipoLocal::Fornecimento)
}

pub fn locais_incidencia(_: &App, p: &Pedido<'_>) -> Saida {
    locais_de(p, TipoLocal::Incidencia)
}

pub fn situacoes_classificacoes(app: &App, p: &Pedido<'_>) -> Saida {
    let nbs = p.obrigatorio("nbs")?;
    let dia = p.data_obrigatoria("data")?;
    exigir_digitos(Some(&nbs), TAM_NBS)?;
    let v = app.com_servicos(|s| s.situacoes_classificacoes_nbs(&nbs, dia))?;
    Ok(Resposta::serializar(StatusCode::OK, &v)?)
}
