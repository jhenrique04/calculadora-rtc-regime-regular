//! logs no formato de console padrão do Spring Boot 3.5 com Micrometer Tracing:
//! `data nível pid --- [app] [thread] [trace-span] logger : mensagem`

use std::cell::RefCell;
use std::io::Write as _;
use std::sync::atomic::{AtomicU8, Ordering};

use chrono::{SecondsFormat, Utc};

pub const APLICACAO: &str = "api-regime-geral";
const LARGURA_THREAD: usize = 15;
const LARGURA_LOGGER: usize = 40;
const LARGURA_CORRELACAO: usize = 49;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Nivel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Off = 5,
}

impl Nivel {
    fn rotulo(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Off => "OFF",
        }
    }

    /// `logging.level.root` aceita os nomes do Logback, sem caixa
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "TRACE" | "ALL" => Some(Self::Trace),
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            "OFF" => Some(Self::Off),
            _ => None,
        }
    }
}

static NIVEL: AtomicU8 = AtomicU8::new(Nivel::Info as u8);

pub fn definir_nivel(n: Nivel) {
    NIVEL.store(n as u8, Ordering::Relaxed);
}

pub fn habilitado(n: Nivel) -> bool {
    n as u8 >= NIVEL.load(Ordering::Relaxed) && n != Nivel::Off
}

thread_local! {
    static CORRELACAO: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

/// trace/span da requisição corrente (MDC `traceId`/`spanId`), restaurado no drop
pub struct Escopo {
    anterior: Option<(String, String)>,
}

impl Escopo {
    pub fn entrar(trace: &str, span: &str) -> Self {
        let anterior = CORRELACAO.with(|c| c.replace(Some((trace.to_string(), span.to_string()))));
        Self { anterior }
    }
}

impl Drop for Escopo {
    fn drop(&mut self) {
        let anterior = self.anterior.take();
        CORRELACAO.with(|c| *c.borrow_mut() = anterior);
    }
}

fn correlacao() -> String {
    CORRELACAO.with(|c| match &*c.borrow() {
        Some((t, s)) => format!("[{t}-{s}] "),
        None => format!("[{}] ", " ".repeat(LARGURA_CORRELACAO)),
    })
}

/// `%15.15t`: alinhado à direita, truncado pela esquerda
fn thread() -> String {
    let atual = std::thread::current();
    let nome = atual.name().unwrap_or("main");
    let inicio = nome.len().saturating_sub(LARGURA_THREAD);
    let nome = nome.get(inicio..).unwrap_or(nome);
    format!("{nome:>LARGURA_THREAD$}")
}

fn logger(nome: &str) -> String {
    let inicio = nome.len().saturating_sub(LARGURA_LOGGER);
    let nome = nome.get(inicio..).unwrap_or(nome);
    format!("{nome:<LARGURA_LOGGER$}")
}

pub fn linha(n: Nivel, nome: &str, msg: &str) -> String {
    format!(
        "{} {:>5} {} --- [{APLICACAO}] [{}] {}{} : {msg}",
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        n.rotulo(),
        std::process::id(),
        thread(),
        correlacao(),
        logger(nome),
    )
}

pub fn registrar(n: Nivel, nome: &str, msg: &str) {
    if !habilitado(n) {
        return;
    }
    let mut out = std::io::stdout().lock();
    // stdout fechado não tem a quem reportar
    let _ = writeln!(out, "{}", linha(n, nome, msg));
}

pub fn debug(nome: &str, msg: &str) {
    registrar(Nivel::Debug, nome, msg);
}

pub fn info(nome: &str, msg: &str) {
    registrar(Nivel::Info, nome, msg);
}

pub fn aviso(nome: &str, msg: &str) {
    registrar(Nivel::Warn, nome, msg);
}

pub fn erro(nome: &str, msg: &str) {
    registrar(Nivel::Error, nome, msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colunas_fixas_como_logback() {
        let l = linha(Nivel::Info, "b.g.s.rtc.CalculadoraTributoApplication", "ok");
        let sem_data = l.split_once(' ').map(|(_, r)| r).unwrap_or_default();
        assert!(sem_data.starts_with(" INFO "));
        assert!(l.contains(&format!("[{}] ", " ".repeat(LARGURA_CORRELACAO))));
        assert!(l.ends_with("b.g.s.rtc.CalculadoraTributoApplication  : ok"));
        let _e = Escopo::entrar("a".repeat(32).as_str(), "b".repeat(16).as_str());
        assert!(linha(Nivel::Error, "x", "y").contains(&format!(
            "[{}-{}] ",
            "a".repeat(32),
            "b".repeat(16)
        )));
    }
}
