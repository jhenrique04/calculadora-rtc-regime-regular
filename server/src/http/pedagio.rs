use hyper::StatusCode;
use rtc_core::modelo::pedagio::PedagioEntrada;
use rtc_core::modelo::validacao::validar_pedagio;
use rtc_core::servico::pedagio::dados_simulados;

use super::App;
use super::erro::ErroHttp;
use super::pedido::Pedido;
use super::resposta::Resposta;

const ASSINATURA: &str = "public org.springframework.http.ResponseEntity<br.gov.serpro.rtc.api.model.output.pedagio.PedagioOutput> br.gov.serpro.rtc.api.controller.PedagioController.calcularTributo(br.gov.serpro.rtc.api.model.input.pedagio.PedagioInput)";
const CABECALHO_AVISO: &str = "x-warning-dados-simulados";
const AVISO_CASO_GERAL: &str = "1";

pub fn calcular(app: &App, p: &Pedido<'_>) -> Result<Resposta, ErroHttp> {
    let e: PedagioEntrada = p.corpo_json(ASSINATURA)?;
    let violacoes = validar_pedagio(&e);
    if !violacoes.is_empty() {
        return Err(ErroHttp::CamposInvalidos(violacoes));
    }
    let saida = app.com_servicos(|s| s.calcular_pedagio(&e))?;
    let r = Resposta::serializar(StatusCode::OK, &saida)?;
    Ok(if dados_simulados(&e) {
        r.com(CABECALHO_AVISO, AVISO_CASO_GERAL)
    } else {
        r
    })
}
