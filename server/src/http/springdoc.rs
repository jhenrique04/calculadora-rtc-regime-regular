//! springdoc-openapi 2.8.9 com swagger-ui 5.21.0. Os documentos OpenAPI são os
//! gerados pela versão Java, com a URL do servidor trocada a cada pedido

use bytes::Bytes;
use hyper::StatusCode;
use hyper::header::{self, HeaderMap};
use rtc_core::json::Escritor;

use super::App;
use super::erro::ErroHttp;
use super::midia::JSON;
use super::pedido::Pedido;
use super::recurso::Recurso;
use super::resposta::{Corpo, Resposta};

pub const OPENAPI_YAML: &str = "application/vnd.oai.openapi";
const MARCADOR_SERVIDOR: &str = "@servidor@";
const API_DOCS_JSON: &str = include_str!("../../assets/springdoc/api-docs.json");
const API_DOCS_YAML: &str = include_str!("../../assets/springdoc/api-docs.yaml");
const INICIALIZADOR: &[u8] = include_bytes!("../../assets/springdoc/swagger-initializer.js");
const NOME_INICIALIZADOR: &str = "swagger-initializer.js";
const VERSAO_WEBJAR: &str = "5.21.0";
const PREFIXO_WEBJARS: &str = "webjars";
const PORTA_HTTP: &str = ":80";

macro_rules! webjar {
    ($($nome:literal),* $(,)?) => {
        &[$(($nome, include_bytes!(concat!("../../assets/swagger-ui/", $nome)) as &[u8])),*]
    };
}

const WEBJAR: &[(&str, &[u8])] = webjar!(
    "favicon-16x16.png",
    "favicon-32x32.png",
    "index.css",
    "index.html",
    "oauth2-redirect.html",
    "swagger-initializer.js",
    "swagger-ui-bundle.js",
    "swagger-ui-bundle.js.map",
    "swagger-ui-es-bundle-core.js",
    "swagger-ui-es-bundle-core.js.map",
    "swagger-ui-es-bundle.js",
    "swagger-ui-es-bundle.js.map",
    "swagger-ui-standalone-preset.js",
    "swagger-ui-standalone-preset.js.map",
    "swagger-ui.css",
    "swagger-ui.css.map",
    "swagger-ui.js",
    "swagger-ui.js.map",
);

type Saida = Result<Resposta, ErroHttp>;

/// `ServletContext.getMimeType` para os arquivos do webjar
fn tipo_mime(nome: &str) -> &'static str {
    match nome.rsplit_once('.').map(|(_, e)| e) {
        Some("png") => "image/png",
        Some("css") => "text/css",
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        _ => "application/octet-stream",
    }
}

fn do_webjar(nome: &str) -> Option<&'static [u8]> {
    WEBJAR.iter().find(|(n, _)| *n == nome).map(|(_, c)| *c)
}

/// `/swagger-ui/**` passa pelo `SwaggerIndexTransformer`; `/webjars/**` serve o original
pub fn recurso(segmentos: &[String]) -> Option<Recurso> {
    // com barra final o arquivo ainda é achado, mas pelo mapeamento genérico
    let (caminho, barra_final): (Vec<&str>, bool) = match segmentos {
        [resto @ .., ultimo] if ultimo.is_empty() => {
            (resto.iter().map(String::as_str).collect(), true)
        }
        _ => (segmentos.iter().map(String::as_str).collect(), false),
    };
    let (nome, conteudo, mapeamento_proprio) = match caminho.as_slice() {
        ["swagger-ui", NOME_INICIALIZADOR] => {
            (NOME_INICIALIZADOR, Some(INICIALIZADOR), !barra_final)
        }
        ["swagger-ui", VERSAO_WEBJAR, NOME_INICIALIZADOR] => {
            (NOME_INICIALIZADOR, Some(INICIALIZADOR), false)
        }
        ["swagger-ui", nome]
        | ["swagger-ui", VERSAO_WEBJAR, nome]
        | ["webjars", "swagger-ui", nome]
        | ["webjars", "swagger-ui", VERSAO_WEBJAR, nome] => (*nome, do_webjar(nome), false),
        _ => return None,
    };
    conteudo.map(|conteudo| Recurso {
        conteudo,
        tipo: tipo_mime(nome),
        mapeamento_proprio,
    })
}

/// caminho do recurso relativo ao padrão que o mapeou, para o `NoResourceFoundException`
pub fn relativo_ao_mapeamento(segmentos: &[String]) -> &[String] {
    match segmentos {
        [primeiro, resto @ ..] if primeiro == PREFIXO_WEBJARS => resto,
        _ => segmentos,
    }
}

/// `scheme://host[:porta]` do pedido; sem `Host`, o Tomcat usa o conector local
fn servidor(h: &HeaderMap, porta: u16) -> String {
    let host = h
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map_or_else(
            || format!("localhost:{porta}"),
            |v| v.strip_suffix(PORTA_HTTP).unwrap_or(v).to_string(),
        );
    format!("http://{host}")
}

fn url_api(app: &App, p: &Pedido<'_>) -> String {
    format!("{}{}", servidor(p.cabecalhos, app.porta), super::CONTEXTO)
}

fn redirecionar(destino: &str) -> Saida {
    Ok(Resposta::new(StatusCode::FOUND).com("location", destino))
}

pub fn raiz(_: &App, _: &Pedido<'_>) -> Saida {
    redirecionar("/api/swagger-ui.html")
}

pub fn swagger_ui_html(_: &App, _: &Pedido<'_>) -> Saida {
    redirecionar("/api/swagger-ui/index.html")
}

/// O controller devolve `byte[]`: tamanho conhecido
fn documento(modelo: &str, tipo: &str, url: &str) -> Resposta {
    Resposta {
        tipo: Some(tipo.into()),
        corpo: Corpo::Fixo(Bytes::from(modelo.replacen(MARCADOR_SERVIDOR, url, 1))),
        conversor: true,
        ..Resposta::new(StatusCode::OK)
    }
}

pub fn api_docs(app: &App, p: &Pedido<'_>) -> Saida {
    Ok(documento(API_DOCS_JSON, JSON, &url_api(app, p)))
}

pub fn api_docs_yaml(app: &App, p: &Pedido<'_>) -> Saida {
    Ok(documento(API_DOCS_YAML, OPENAPI_YAML, &url_api(app, p)))
}

/// `SwaggerConfigResource`: mapa serializado pelo Jackson
pub fn configuracao(app: &App, p: &Pedido<'_>) -> Saida {
    let redirecionamento = format!("{}/swagger-ui/oauth2-redirect.html", url_api(app, p));
    let texto = |e: &mut Escritor, nome: &str, valor: &str| {
        e.nome(nome);
        e.valor_string(valor);
    };
    let mut e = Escritor::new();
    e.inicio_objeto();
    texto(&mut e, "configUrl", "/api/api-docs/swagger-config");
    e.nome("displayRequestDuration");
    e.valor_bruto("true");
    texto(&mut e, "layout", "BaseLayout");
    texto(&mut e, "oauth2RedirectUrl", &redirecionamento);
    texto(&mut e, "operationsSorter", "method");
    e.nome("supportedSubmitMethods");
    e.valor_bruto("[\"get\",\"post\"]");
    texto(&mut e, "url", "/api/api-docs");
    texto(&mut e, "validatorUrl", "");
    e.fim_objeto();
    Ok(Resposta::json(StatusCode::OK, e.finalizar()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segmentos(c: &str) -> Vec<String> {
        c.split('/').map(String::from).collect()
    }

    #[test]
    fn recursos_do_webjar() {
        let tamanho = |c: &str| recurso(&segmentos(c)).map(|r| (r.conteudo.len(), r.tipo));
        assert_eq!(tamanho("swagger-ui/index.css"), Some((202, "text/css")));
        assert_eq!(tamanho("swagger-ui/index.css/"), Some((202, "text/css")));
        assert_eq!(
            tamanho("swagger-ui/5.21.0/swagger-initializer.js"),
            Some((707, "text/javascript"))
        );
        assert_eq!(
            tamanho("swagger-ui/swagger-initializer.js"),
            Some((707, "text/javascript"))
        );
        assert_eq!(
            tamanho("webjars/swagger-ui/swagger-initializer.js"),
            Some((539, "text/javascript"))
        );
        assert_eq!(
            tamanho("webjars/swagger-ui/5.21.0/swagger-ui.js.map").map(|t| t.1),
            Some("application/octet-stream")
        );
        for ausente in [
            "swagger-ui",
            "swagger-ui/",
            "swagger-ui/INDEX.CSS",
            "webjars/swagger-ui/9.9.9/index.css",
        ] {
            assert!(recurso(&segmentos(ausente)).is_none(), "{ausente}");
        }
    }

    #[test]
    fn servidor_como_springdoc() {
        let com_host = |v: &str| {
            let mut h = HeaderMap::new();
            h.insert(header::HOST, v.parse().unwrap_or_else(|_| unreachable!()));
            servidor(&h, 8080)
        };
        assert_eq!(com_host("exemplo:80"), "http://exemplo");
        assert_eq!(com_host("exemplo:8080"), "http://exemplo:8080");
        assert_eq!(com_host("[::1]:5"), "http://[::1]:5");
        assert_eq!(servidor(&HeaderMap::new(), 8080), "http://localhost:8080");
    }

    #[test]
    fn modelos_tem_um_marcador() {
        for modelo in [API_DOCS_JSON, API_DOCS_YAML] {
            assert_eq!(modelo.matches(MARCADOR_SERVIDOR).count(), 1);
        }
    }
}
