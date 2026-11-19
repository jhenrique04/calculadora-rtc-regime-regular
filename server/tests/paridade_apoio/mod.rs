//! apoio do teste de paridade: banco, servidor, cliente HTTP que reproduz o
//! `urllib` usado na gravação e a normalização do `comparar.py`

mod http;
mod normalizacao;

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

pub use http::enviar;
pub use normalizacao::comparar;

/// porta do container Java na gravação; aparece em URLs geradas pelo servidor
const PORTA_GRAVACAO: &str = "18100";
const ESPERA_SERVIDOR: Duration = Duration::from_secs(60);
const INTERVALO_SONDAGEM: Duration = Duration::from_millis(100);
// o que o serviço remoto respondia quando as respostas foram gravadas
const VERSAO_REMOTA: &str =
    r#"{"versaoApp":"1.7.1-2beedf45","versaoDb":"V0073","dataVersaoDb":"2026-11-09"}"#;

pub struct Esperado {
    pub status: u16,
    pub cabecalhos: Vec<(String, String)>,
    pub corpo: Option<String>,
    pub corpo_crc32: Option<u32>,
    pub corpo_tamanho: Option<usize>,
}

pub struct Caso {
    pub id: String,
    pub metodo: String,
    pub caminho: String,
    pub cabecalhos: Vec<(String, String)>,
    pub corpo: Option<Vec<u8>>,
    pub esperado: Esperado,
}

pub struct Suite {
    pub seguir_redirecionamento: bool,
    pub casos: Vec<Caso>,
}

fn texto(v: &Value, campo: &str) -> String {
    v[campo].as_str().unwrap_or_default().to_string()
}

fn pares(v: &Value) -> Vec<(String, String)> {
    match v {
        Value::Object(m) => m
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
            .collect(),
        Value::Array(l) => l
            .iter()
            .map(|p| (texto_de(&p[0]), texto_de(&p[1])))
            .collect(),
        _ => Vec::new(),
    }
}

fn texto_de(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or_else(|e| panic!("hex {s}: {e}")))
        .collect()
}

fn caso(v: &Value) -> Caso {
    let e = &v["esperado"];
    Caso {
        id: texto(v, "id"),
        metodo: texto(v, "metodo"),
        caminho: texto(v, "caminho"),
        cabecalhos: pares(&v["cabecalhos"]),
        corpo: v["corpo_hex"].as_str().map(hex),
        esperado: Esperado {
            status: e["status"]
                .as_u64()
                .and_then(|s| u16::try_from(s).ok())
                .unwrap_or_default(),
            cabecalhos: pares(&e["cabecalhos"]),
            corpo: e["corpo"].as_str().map(str::to_string),
            corpo_crc32: e["corpo_crc32"]
                .as_u64()
                .and_then(|c| u32::try_from(c).ok()),
            corpo_tamanho: e["corpo_tamanho"]
                .as_u64()
                .and_then(|t| usize::try_from(t).ok()),
        },
    }
}

impl Suite {
    pub fn carregar(arquivo: &Path) -> Self {
        let bruto = std::fs::read(arquivo).unwrap_or_else(|e| panic!("{}: {e}", arquivo.display()));
        let mut json = String::new();
        flate2::read::GzDecoder::new(bruto.as_slice())
            .read_to_string(&mut json)
            .unwrap_or_else(|e| panic!("{}: {e}", arquivo.display()));
        let doc: Value =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{}: {e}", arquivo.display()));
        Self {
            seguir_redirecionamento: doc["seguir_redirecionamento"].as_bool().unwrap_or(true),
            casos: doc["casos"]
                .as_array()
                .map(|l| l.iter().map(caso).collect())
                .unwrap_or_default(),
        }
    }
}

/// porta livre com cinco dígitos, como a da gravação (tamanhos de corpo dependem disso)
fn porta_livre() -> u16 {
    loop {
        let l = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("bind: {e}"));
        let porta = l.local_addr().map(|a| a.port()).unwrap_or_default();
        if porta.to_string().len() == PORTA_GRAVACAO.len() {
            return porta;
        }
    }
}

/// serviço remoto de versão (`VERSAO_OFFLINE_URL`) com a resposta da época da gravação
fn versao_remota() -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("bind: {e}"));
    let porta = l.local_addr().map(|a| a.port()).unwrap_or_default();
    std::thread::spawn(move || {
        for conexao in l.incoming().flatten() {
            responder_versao(conexao);
        }
    });
    format!("http://127.0.0.1:{porta}/versao")
}

fn responder_versao(conexao: TcpStream) {
    let mut leitor = BufReader::new(&conexao);
    let mut linha = String::new();
    while leitor.read_line(&mut linha).is_ok_and(|n| n > 0) && linha != "\r\n" {
        linha.clear();
    }
    let resposta = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{VERSAO_REMOTA}",
        VERSAO_REMOTA.len()
    );
    if let Err(e) = (&conexao).write_all(resposta.as_bytes()) {
        eprintln!("mock de versão: {e}");
    }
}

fn gerar_banco(raiz: &Path) {
    let dir_db = raiz.join("calculadora/db");
    std::fs::create_dir_all(&dir_db).unwrap_or_else(|e| panic!("{}: {e}", dir_db.display()));
    let sql = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../flyway/sql");
    rtc_banco::migrar(&sql, &dir_db.join("calculadora-pro.db"))
        .unwrap_or_else(|e| panic!("gerar banco: {e}"));
}

pub struct Ambiente {
    pub porta: u16,
    raiz: PathBuf,
    servidor: Child,
}

impl Ambiente {
    pub fn subir() -> Self {
        let raiz = std::env::temp_dir().join(format!("rtc-paridade-{}", std::process::id()));
        if raiz.exists() {
            std::fs::remove_dir_all(&raiz).unwrap_or_else(|e| panic!("{}: {e}", raiz.display()));
        }
        gerar_banco(&raiz);
        let porta = porta_livre();
        let servidor = Command::new(env!("CARGO_BIN_EXE_rtc-server"))
            .current_dir(&raiz)
            .env_clear()
            .env("TZ", "UTC")
            .env("SERVER_PORT", porta.to_string())
            .env("MANAGEMENT_SERVER_PORT", porta_livre().to_string())
            .env("VERSAO_OFFLINE_URL", versao_remota())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("subir rtc-server: {e}"));
        let ambiente = Self {
            porta,
            raiz,
            servidor,
        };
        ambiente.aguardar();
        ambiente
    }

    fn aguardar(&self) {
        let inicio = Instant::now();
        while TcpStream::connect(("127.0.0.1", self.porta)).is_err() {
            assert!(inicio.elapsed() < ESPERA_SERVIDOR, "rtc-server não subiu");
            std::thread::sleep(INTERVALO_SONDAGEM);
        }
    }

    /// inverso de `local`, para comparar com o crc32 calculado na gravação
    pub fn gravado(&self, obtido: &str) -> String {
        obtido.replace(
            &format!("localhost:{}", self.porta),
            &format!("localhost:{PORTA_GRAVACAO}"),
        )
    }

    /// valor gravado com a porta do container trocada pela do servidor de teste
    pub fn local(&self, gravado: &str) -> String {
        gravado.replace(
            &format!("localhost:{PORTA_GRAVACAO}"),
            &format!("localhost:{}", self.porta),
        )
    }
}

impl Drop for Ambiente {
    fn drop(&mut self) {
        if let Err(e) = self.servidor.kill() {
            eprintln!("parar rtc-server: {e}");
        }
        if let Err(e) = self.servidor.wait() {
            eprintln!("aguardar rtc-server: {e}");
        }
        if let Err(e) = std::fs::remove_dir_all(&self.raiz) {
            eprintln!("remover {}: {e}", self.raiz.display());
        }
    }
}
