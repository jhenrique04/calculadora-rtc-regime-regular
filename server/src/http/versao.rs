use hyper::StatusCode;
use hyper::header::ACCEPT;
use rtc_core::erro::{Falha, Resultado};
use rtc_core::json::ler_documento;
use rtc_core::servico::download::{DownloadUrlSaida, resolver};
use rtc_core::servico::versao_offline::{VersaoOfflineSaida, VersaoRemota};

use super::erro::ErroHttp;
use super::pedido::Pedido;
use super::resposta::Resposta;
use super::{App, VERSAO_APLICACAO};
use crate::log;

const LOGGER_VERSAO: &str = "b.g.s.r.d.s.o.VersaoOfflineService";
const TENTATIVAS: u32 = 3;
const ESPERA_ENTRE_TENTATIVAS: std::time::Duration = std::time::Duration::from_secs(2);
// accept que o RestTemplate manda para um alvo Jackson
const ACCEPT_JSON: &str = "application/json, application/*+json";
const LARGURA_BANNER: usize = 50;

type Saida = Result<Resposta, ErroHttp>;

fn erro_io(url: &str, e: impl std::fmt::Display) -> Falha {
    Falha::sistema(format!("I/O error on GET request for \"{url}\": {e}"))
}

/// `restTemplate.getForObject`: corpo vazio vira `null`
fn consultar_remota(url: &str) -> Resultado<Option<VersaoRemota>> {
    let cliente = reqwest::blocking::Client::builder()
        .build()
        .map_err(|e| erro_io(url, e))?;
    let resposta = cliente
        .get(url)
        .header(ACCEPT, ACCEPT_JSON)
        .send()
        .map_err(|e| erro_io(url, e))?;
    let status = resposta.status();
    let corpo = resposta.bytes().map_err(|e| erro_io(url, e))?;
    if !status.is_success() {
        let motivo = status.canonical_reason().unwrap_or_default();
        let texto = if corpo.is_empty() {
            "[no body]".to_string()
        } else {
            format!("\"{}\"", String::from_utf8_lossy(&corpo))
        };
        return Err(Falha::sistema(format!(
            "{} {motivo}: {texto}",
            status.as_u16()
        )));
    }
    if corpo.is_empty() {
        return Ok(None);
    }
    ler_documento(&corpo).map_err(|e| {
        Falha::sistema(format!(
            "Error while extracting response for type [class br.gov.serpro.rtc.api.model.output.VersaoOutput] and content type [application/json]: {e}"
        ))
    })
}

fn verificar(app: &App) -> Resultado<VersaoOfflineSaida> {
    let local = app.com_servicos(|s| s.versao_local(VERSAO_APLICACAO))?;
    let remota = consultar_remota(&app.url_versao)?;
    Ok(VersaoOfflineSaida::comparar(local, remota))
}

pub fn status(app: &App, _: &Pedido<'_>) -> Saida {
    Ok(Resposta::serializar(StatusCode::OK, &verificar(app)?)?)
}

fn banner(titulo: &str) -> String {
    let prefixo = format!("= {titulo} ");
    let resto = LARGURA_BANNER.saturating_sub(prefixo.chars().count());
    format!("{prefixo}{}", "=".repeat(resto))
}

fn exibir_status(s: &VersaoOfflineSaida) {
    let (atualizados, desatualizados) = s.linhas_status();
    for (titulo, linhas) in [
        ("VERSÃO ATUALIZADA", atualizados),
        ("VERSÃO DESATUALIZADA", desatualizados),
    ] {
        if linhas.is_empty() {
            continue;
        }
        log::info(LOGGER_VERSAO, &banner(titulo));
        for l in linhas {
            log::info(LOGGER_VERSAO, &l);
        }
    }
}

/// `@Retryable` do evento `ApplicationReadyEvent`, com o `@Recover` no fim
pub fn verificar_na_inicializacao(app: &App) {
    let mut ultimo_erro = String::new();
    for tentativa in 1..=TENTATIVAS {
        match verificar(app) {
            Ok(s) => return exibir_status(&s),
            Err(f) => ultimo_erro = f.mensagem.unwrap_or_else(|| "null".into()),
        }
        if tentativa < TENTATIVAS {
            std::thread::sleep(ESPERA_ENTRE_TENTATIVAS);
        }
    }
    log::aviso(
        LOGGER_VERSAO,
        &format!(
            "Não foi possível verificar versão remota após {TENTATIVAS} tentativas: {ultimo_erro}"
        ),
    );
}

pub fn download_url(app: &App, p: &Pedido<'_>) -> Saida {
    let plataforma = p.obrigatorio("platform")?;
    let r = resolver(Some(&plataforma), &app.plataformas_download)?;
    if r.url.trim().is_empty() {
        return Ok(Resposta::new(StatusCode::NO_CONTENT));
    }
    app.metricas.contar_download(r.plataforma);
    let saida = DownloadUrlSaida {
        download_url: Some(r.url.to_string()),
    };
    Ok(Resposta::serializar(StatusCode::OK, &saida)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_com_largura_fixa() {
        assert_eq!(
            banner("VERSÃO ATUALIZADA"),
            format!("= VERSÃO ATUALIZADA {}", "=".repeat(30))
        );
        assert_eq!(
            banner("VERSÃO DESATUALIZADA"),
            format!("= VERSÃO DESATUALIZADA {}", "=".repeat(27))
        );
    }
}
