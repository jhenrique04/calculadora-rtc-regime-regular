//! API do regime regular da calculadora RTC (porta do `CalculadorTributo`
//! Spring Boot): servidor de aplicação em `/api` e servidor de gestão

mod config;
mod http;
mod log;
mod metricas;

use std::future::Future;
use std::net::{Ipv6Addr, SocketAddr};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::server::graceful::GracefulShutdown;
use rtc_core::db::Pool;
use tokio::net::TcpListener;

use config::Config;
use http::{Ambiente, App, CorpoHttp};
use metricas::Metricas;

const LOGGER_APLICACAO: &str = "b.g.s.rtc.CalculadoraTributoApplication";
const LOGGER_DB: &str = "b.g.s.r.config.db.DatabaseValidator";
const LOGGER_TOMCAT: &str = "o.s.b.w.embedded.tomcat.TomcatWebServer";
const LOGGER_GESTAO: &str = "o.s.b.a.e.web.EndpointLinksResolver";
const LOGGER_DOWNLOAD: &str = "b.g.s.r.c.d.DownloadPlatformProperties";
const LOGGER_SHUTDOWN: &str = "o.s.b.w.e.tomcat.GracefulShutdown";
const TIMEOUT_SHUTDOWN: Duration = Duration::from_secs(30);
const TIMEOUT_CABECALHO: Duration = Duration::from_secs(60);

fn main() -> ExitCode {
    let inicio = Instant::now();
    let cfg = match Config::carregar() {
        Ok(c) => c,
        Err(e) => return falhar(&e.to_string()),
    };
    log::definir_nivel(cfg.nivel_log);
    let contador = AtomicUsize::new(0);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name_fn(move || {
            let n = contador.fetch_add(1, Ordering::Relaxed) + 1;
            format!("http-nio-8080-exec-{n}")
        })
        .build();
    let runtime = match runtime {
        Ok(r) => r,
        Err(e) => return falhar(&e.to_string()),
    };
    match runtime.block_on(executar(cfg, inicio)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => falhar(&e),
    }
}

fn falhar(erro: &str) -> ExitCode {
    log::erro(LOGGER_APLICACAO, &format!("Application run failed\n{erro}"));
    ExitCode::FAILURE
}

async fn ouvir(porta: u16) -> Result<TcpListener, String> {
    TcpListener::bind(SocketAddr::from((Ipv6Addr::UNSPECIFIED, porta)))
        .await
        .map_err(|e| format!("Web server failed to start. Port {porta} was already in use. ({e})"))
}

async fn executar(cfg: Config, inicio: Instant) -> Result<(), String> {
    log::info(
        LOGGER_APLICACAO,
        &format!(
            "Starting CalculadoraTributoApplication v{} with PID {}",
            http::VERSAO_APLICACAO,
            std::process::id()
        ),
    );
    log::info(
        LOGGER_APLICACAO,
        "The following 1 profile is active: \"offline\"",
    );
    log::info(
        LOGGER_DB,
        &format!(">>> Banco de dados em uso: [{}] <<<", cfg.arquivo_db),
    );
    let pool = Pool::abrir(&cfg.caminho_db).map_err(|e| e.to_string())?;
    let metricas = Arc::new(Metricas::new());
    registrar_plataformas(&cfg.plataformas_download);
    let ambiente = Ambiente {
        url_versao: cfg.url_versao.clone(),
        plataformas_download: cfg.plataformas_download.clone(),
        porta: cfg.porta,
    };
    let app = Arc::new(App::new(pool, Arc::clone(&metricas), ambiente).map_err(|e| e.to_string())?);

    let api = ouvir(cfg.porta).await?;
    log::info(
        LOGGER_TOMCAT,
        &format!(
            "Tomcat started on port {} (http) with context path '/api'",
            cfg.porta
        ),
    );
    let gestao = ouvir(cfg.porta_gestao).await?;
    log::info(LOGGER_GESTAO, "Exposing 2 endpoints beneath base path ''");
    log::info(
        LOGGER_TOMCAT,
        &format!(
            "Tomcat started on port {} (http) with context path '/'",
            cfg.porta_gestao
        ),
    );
    let segundos = inicio.elapsed().as_secs_f64();
    log::info(
        LOGGER_APLICACAO,
        &format!("Started CalculadoraTributoApplication in {segundos:.3} seconds"),
    );
    metricas.marcar_pronto();
    verificar_versao_em_segundo_plano(Arc::clone(&app));

    let graceful = GracefulShutdown::new();
    let app_srv = {
        let app = Arc::clone(&app);
        move |req: Request<Incoming>| http::atender(Arc::clone(&app), req)
    };
    let gestao_srv = {
        let m = Arc::clone(&metricas);
        move |req: Request<Incoming>| http::atender_gestao(Arc::clone(&m), req)
    };
    let sinal = desligamento();
    tokio::pin!(sinal);
    loop {
        tokio::select! {
            r = api.accept() => aceitar(r, &graceful, app_srv.clone()),
            r = gestao.accept() => aceitar(r, &graceful, gestao_srv.clone()),
            () = &mut sinal => break,
        }
    }
    log::info(
        LOGGER_SHUTDOWN,
        "Commencing graceful shutdown. Waiting for active requests to complete",
    );
    match tokio::time::timeout(TIMEOUT_SHUTDOWN, graceful.shutdown()).await {
        Ok(()) => log::info(LOGGER_SHUTDOWN, "Graceful shutdown complete"),
        Err(_) => log::info(
            LOGGER_SHUTDOWN,
            "Graceful shutdown aborted with one or more requests still active",
        ),
    }
    Ok(())
}

fn registrar_plataformas(plataformas: &[(String, String)]) {
    if plataformas.is_empty() {
        log::aviso(
            LOGGER_DOWNLOAD,
            "Nenhuma plataforma de download configurada em 'application.download.platforms'",
        );
        return;
    }
    for (nome, url) in plataformas {
        if url.trim().is_empty() {
            log::info(
                LOGGER_DOWNLOAD,
                &format!("Plataforma de download '{nome}' sem URL configurada"),
            );
        }
    }
    let nomes: Vec<&str> = plataformas.iter().map(|(n, _)| n.as_str()).collect();
    log::info(
        LOGGER_DOWNLOAD,
        &format!(
            "Plataformas de download configuradas: [{}]",
            nomes.join(", ")
        ),
    );
}

/// no Java roda na thread `main` depois do `ApplicationReadyEvent`; aqui numa
/// thread própria com o mesmo nome, que termina após no máximo 3 tentativas
fn verificar_versao_em_segundo_plano(app: Arc<App>) {
    let r = std::thread::Builder::new()
        .name("main".into())
        .spawn(move || http::verificar_na_inicializacao(&app));
    if let Err(e) = r {
        log::aviso(
            LOGGER_APLICACAO,
            &format!("falha ao verificar versão remota: {e}"),
        );
    }
}

fn aceitar<S, F>(
    r: std::io::Result<(tokio::net::TcpStream, SocketAddr)>,
    graceful: &GracefulShutdown,
    servico: S,
) where
    S: Fn(Request<Incoming>) -> F + Send + 'static,
    F: Future<Output = Result<Response<CorpoHttp>, std::convert::Infallible>> + Send + 'static,
{
    let (stream, _) = match r {
        Ok(c) => c,
        Err(e) => {
            log::aviso(LOGGER_TOMCAT, &format!("falha ao aceitar conexão: {e}"));
            return;
        }
    };
    let conn = http1::Builder::new()
        .timer(TokioTimer::new())
        .header_read_timeout(TIMEOUT_CABECALHO)
        .title_case_headers(true)
        .serve_connection(TokioIo::new(stream), service_fn(servico));
    let conn = graceful.watch(conn);
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            log::debug(LOGGER_TOMCAT, &format!("conexão encerrada com erro: {e}"));
        }
    });
}

async fn desligamento() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            log::erro(
                LOGGER_SHUTDOWN,
                &format!("falha ao instalar handler de SIGINT: {e}"),
            );
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let term = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                log::erro(
                    LOGGER_SHUTDOWN,
                    &format!("falha ao instalar handler de SIGTERM: {e}"),
                );
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {}
        () = term => {}
    }
}
