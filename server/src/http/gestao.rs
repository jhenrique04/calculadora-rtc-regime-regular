//! servidor de gestão do Actuator (`management.server.port`): links, health e
//! Prometheus em `/metrics`

use std::convert::Infallible;
use std::sync::Arc;

use bytes::Bytes;
use hyper::body::Incoming;
use hyper::header::{self, HeaderMap};
use hyper::{Method, Request, Response, StatusCode};

use super::resposta::{Corpo, Resposta};
use super::{CorpoHttp, Saida, erro_boot, finalizar};
use crate::metricas::Metricas;

const ACTUATOR_JSON: &str = "application/vnd.spring-boot.actuator.v3+json";
const PROMETHEUS: &str = "text/plain;version=0.0.4;charset=utf-8";
const HEALTH_UP: &str = r#"{"status":"UP"}"#;

fn host(h: &HeaderMap) -> &str {
    h.get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost")
}

fn links(base: &str) -> String {
    format!(
        concat!(
            r#"{{"_links":{{"self":{{"href":"{b}","templated":false}},"#,
            r#""health":{{"href":"{b}/health","templated":false}},"#,
            r#""health-path":{{"href":"{b}/health/{{*path}}","templated":true}},"#,
            r#""prometheus":{{"href":"{b}/metrics","templated":false}}}}}}"#
        ),
        b = base
    )
}

fn actuator(corpo: String) -> Resposta {
    Resposta {
        tipo: Some(ACTUATOR_JSON.into()),
        ..Resposta::json(StatusCode::OK, corpo)
    }
}

fn rotear(metricas: &Metricas, metodo: &Method, caminho: &str, h: &HeaderMap) -> Resposta {
    if *metodo != Method::GET && *metodo != Method::HEAD {
        return erro_boot(StatusCode::METHOD_NOT_ALLOWED, caminho);
    }
    match caminho {
        "/" | "" => actuator(links(&format!("http://{}", host(h)))),
        "/health" => actuator(HEALTH_UP.into()),
        p if p.starts_with("/health/") => erro_boot(StatusCode::NOT_FOUND, caminho),
        "/metrics" => Resposta {
            tipo: Some(PROMETHEUS.into()),
            corpo: Corpo::Fluxo(Bytes::from(metricas.exportar())),
            ..Resposta::new(StatusCode::OK)
        },
        _ => erro_boot(StatusCode::NOT_FOUND, caminho),
    }
}

pub async fn atender(
    metricas: Arc<Metricas>,
    req: Request<Incoming>,
) -> Result<Response<CorpoHttp>, Infallible> {
    let r = rotear(&metricas, req.method(), req.uri().path(), req.headers());
    let saida = Saida {
        filtrada: true,
        ..Saida::tomcat(r)
    };
    Ok(finalizar(saida, req.headers(), req.uri().path()))
}
