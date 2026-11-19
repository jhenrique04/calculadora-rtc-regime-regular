//! métricas no formato do registry Prometheus do Micrometer para os medidores
//! que existem fora da JVM: `http_server_requests_*`, `application_*` e `process_*`

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CLASSE_PRINCIPAL: &str = "br.gov.serpro.rtc.CalculadoraTributoApplication";
// timeWindowMax do Micrometer: janela de 2 min em 3 baldes
const JANELA_MAX: Duration = Duration::from_secs(120);
const BALDES_MAX: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rotulos {
    pub excecao: String,
    pub metodo: String,
    pub status: u16,
    pub uri: String,
}

impl Rotulos {
    fn resultado(&self) -> &'static str {
        match self.status {
            100..=199 => "INFORMATIONAL",
            200..=299 => "SUCCESS",
            300..=399 => "REDIRECTION",
            400..=499 => "CLIENT_ERROR",
            500..=599 => "SERVER_ERROR",
            _ => "UNKNOWN",
        }
    }

    fn texto(&self, com_erro: bool) -> String {
        let erro = if com_erro {
            format!("error=\"{}\",", self.excecao)
        } else {
            String::new()
        };
        format!(
            "{erro}exception=\"{}\",method=\"{}\",outcome=\"{}\",status=\"{}\",uri=\"{}\"",
            self.excecao,
            self.metodo,
            self.resultado(),
            self.status,
            self.uri
        )
    }
}

#[derive(Debug, Default)]
struct Temporizador {
    contagem: u64,
    soma: f64,
    baldes: [f64; BALDES_MAX as usize],
    rotacao: Option<Instant>,
}

impl Temporizador {
    fn girar(&mut self, agora: Instant) {
        let passo = JANELA_MAX / BALDES_MAX;
        let inicio = *self.rotacao.get_or_insert(agora);
        let passos = agora.duration_since(inicio).as_nanos() / passo.as_nanos().max(1);
        if passos == 0 {
            return;
        }
        let n = usize::try_from(passos)
            .unwrap_or(usize::MAX)
            .min(self.baldes.len());
        self.baldes.rotate_left(n);
        let tam = self.baldes.len();
        for b in &mut self.baldes[tam - n..] {
            *b = 0.0;
        }
        self.rotacao = Some(agora);
    }

    fn registrar(&mut self, d: Duration, agora: Instant) {
        self.girar(agora);
        let s = d.as_secs_f64();
        self.contagem += 1;
        self.soma += s;
        for b in &mut self.baldes {
            *b = b.max(s);
        }
    }

    fn maximo(&mut self, agora: Instant) -> f64 {
        self.girar(agora);
        self.baldes[0]
    }
}

pub struct Metricas {
    inicio: Instant,
    inicio_unix: f64,
    pronto_em: Mutex<Option<f64>>,
    requisicoes: Mutex<BTreeMap<Rotulos, Temporizador>>,
    /// `api_calc_downloads_total` por plataforma
    downloads: Mutex<BTreeMap<String, u64>>,
}

impl Default for Metricas {
    fn default() -> Self {
        Self::new()
    }
}

fn travar<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match m.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

/// valores como o `Double.toString` do Java (sempre com parte decimal)
fn double(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e7 {
        format!("{v:.1}")
    } else {
        v.to_string()
    }
}

impl Metricas {
    pub fn new() -> Self {
        let inicio_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64());
        Self {
            inicio: Instant::now(),
            inicio_unix,
            pronto_em: Mutex::new(None),
            requisicoes: Mutex::new(BTreeMap::new()),
            downloads: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn marcar_pronto(&self) {
        *travar(&self.pronto_em) = Some(self.inicio.elapsed().as_secs_f64());
    }

    pub fn registrar(&self, r: Rotulos, d: Duration) {
        travar(&self.requisicoes)
            .entry(r)
            .or_default()
            .registrar(d, Instant::now());
    }

    pub fn contar_download(&self, plataforma: &str) {
        *travar(&self.downloads)
            .entry(plataforma.to_string())
            .or_default() += 1;
    }

    fn exportar_downloads(&self, s: &mut String) {
        let downloads = travar(&self.downloads);
        if downloads.is_empty() {
            return;
        }
        s.push_str("# HELP api_calc_downloads_total Total number of download URL requests\n# TYPE api_calc_downloads_total counter\n");
        for (plataforma, n) in downloads.iter() {
            let _ = writeln!(
                s,
                "api_calc_downloads_total{{endpoint=\"download-url\",platform=\"{plataforma}\"}} {n}.0"
            );
        }
    }

    pub fn exportar(&self) -> String {
        let mut s = String::new();
        if let Some(p) = *travar(&self.pronto_em) {
            for (nome, ajuda) in [
                (
                    "application_ready_time_seconds",
                    "Time taken for the application to be ready to service requests",
                ),
                (
                    "application_started_time_seconds",
                    "Time taken to start the application",
                ),
            ] {
                let _ = write!(
                    s,
                    "# HELP {nome} {ajuda}\n# TYPE {nome} gauge\n{nome}{{main_application_class=\"{CLASSE_PRINCIPAL}\"}} {p:.3}\n"
                );
            }
        }
        self.exportar_downloads(&mut s);
        self.exportar_http(&mut s);
        let uptime = self.inicio.elapsed().as_secs_f64();
        let _ = write!(
            s,
            "# HELP process_start_time_seconds Start time of the process since unix epoch.\n# TYPE process_start_time_seconds gauge\nprocess_start_time_seconds {}\n# HELP process_uptime_seconds The uptime of the Java virtual machine\n# TYPE process_uptime_seconds gauge\nprocess_uptime_seconds {uptime:.3}\n",
            double(self.inicio_unix)
        );
        s
    }

    fn exportar_http(&self, s: &mut String) {
        let agora = Instant::now();
        let mut req = travar(&self.requisicoes);
        if req.is_empty() {
            return;
        }
        s.push_str(
            "# HELP http_server_requests_seconds  \n# TYPE http_server_requests_seconds summary\n",
        );
        for (r, t) in req.iter() {
            let rot = r.texto(true);
            let _ = writeln!(
                s,
                "http_server_requests_seconds_count{{{rot}}} {}",
                t.contagem
            );
            let _ = writeln!(s, "http_server_requests_seconds_sum{{{rot}}} {}", t.soma);
        }
        s.push_str("# HELP http_server_requests_seconds_max  \n# TYPE http_server_requests_seconds_max gauge\n");
        for (r, t) in req.iter_mut() {
            let _ = writeln!(
                s,
                "http_server_requests_seconds_max{{{}}} {}",
                r.texto(true),
                double(t.maximo(agora))
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exporta_requisicoes_com_rotulos_do_micrometer() {
        let m = Metricas::new();
        m.registrar(
            Rotulos {
                excecao: "none".into(),
                metodo: "POST".into(),
                status: 200,
                uri: "/calculadora/regime-geral".into(),
            },
            Duration::from_millis(5),
        );
        let out = m.exportar();
        assert!(out.contains("http_server_requests_seconds_count{error=\"none\",exception=\"none\",method=\"POST\",outcome=\"SUCCESS\",status=\"200\",uri=\"/calculadora/regime-geral\"} 1"));
        assert!(out.contains("process_uptime_seconds"));
    }
}
