//! exceções que chegam ao `ApiExceptionHandler`/`ObservabilidadeExceptionHandler`
//! e o `ProblemDetail` que cada uma produz

use hyper::StatusCode;
use rtc_core::erro::{Classe, Falha};
use rtc_core::json::{ErroEscrita, ErroJson, Escrever, Escritor};
use rtc_core::modelo::validacao::Violacao;
use rtc_core::servico::observabilidade::ErroDetalhe;

use super::resposta::Problema;

const TITULO_CAMPOS_INVALIDOS: &str = "Campos inválidos.";
const TITULO_TIPO_ARGUMENTO: &str = "Tipo de argumento inválido";
const PATH_TIPO_ARGUMENTO: &str = "tipo-argumento-invalido";
const MSG_VALIDACAO_METODO: &str = "Validation failure";
const TITULO_OBSERVABILIDADE: &str = "Erros no processamento do cálculo";

#[derive(Debug)]
pub enum ErroHttp {
    /// exceção de serviço (hierarquia de `Classe`)
    Falha(Falha),
    /// `HttpMessageNotReadableException` por corpo ausente
    CorpoAusente(&'static str),
    /// `HttpMessageNotReadableException` do Jackson
    Json(ErroJson),
    CamposInvalidos(Vec<Violacao>),
    ParametroAusente(&'static str),
    TipoInvalido {
        nome: &'static str,
        valor: String,
    },
    /// `HandlerMethodValidationException` (restrição em `@RequestParam`)
    ValidacaoMetodo,
    Observabilidade(Vec<ErroDetalhe>),
    Escrita(ErroEscrita),
}

impl From<Falha> for ErroHttp {
    fn from(f: Falha) -> Self {
        Self::Falha(f)
    }
}

impl From<ErroEscrita> for ErroHttp {
    fn from(e: ErroEscrita) -> Self {
        Self::Escrita(e)
    }
}

fn status(codigo: u16) -> StatusCode {
    StatusCode::from_u16(codigo).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

fn tipo(base_url: &str, path: &str) -> String {
    format!("{base_url}/errors/{path}")
}

fn sistema(base_url: &str, st: StatusCode, detalhe: Option<String>, instancia: &str) -> Problema {
    let p = Classe::Sistema.problema();
    Problema {
        tipo: tipo(base_url, p.path),
        titulo: p.titulo.to_string(),
        status: st,
        detalhe,
        instancia: instancia.to_string(),
        com_timestamp: true,
        erros: None,
    }
}

fn detalhe_campos(violacoes: &[Violacao]) -> String {
    let mut s = String::from("Um ou mais campos são inválidos - ");
    for v in violacoes {
        s.push_str(&format!("[{}: {}] ", v.caminho, v.mensagem));
    }
    s
}

fn lista_erros(erros: &[ErroDetalhe]) -> Result<String, ErroEscrita> {
    let mut e = Escritor::new();
    e.inicio_lista();
    for d in erros {
        d.escrever(&mut e)?;
    }
    e.fim_lista();
    Ok(e.finalizar())
}

impl ErroHttp {
    pub fn mensagem(&self) -> String {
        match self {
            Self::Falha(f) => f.to_string(),
            Self::CorpoAusente(assinatura) => {
                format!("Required request body is missing: {assinatura}")
            }
            Self::Json(e) => format!("JSON parse error: {}", e.mensagem),
            Self::CamposInvalidos(v) => detalhe_campos(v),
            Self::ParametroAusente(nome) => {
                format!("Required request parameter '{nome}' is not present")
            }
            Self::TipoInvalido { nome, valor } => format!("Parâmetro inválido [{nome}: {valor}] "),
            Self::ValidacaoMetodo => format!("400 BAD_REQUEST \"{MSG_VALIDACAO_METODO}\""),
            Self::Observabilidade(_) => "Erros de cálculo encontrados".into(),
            Self::Escrita(e) => format!("Could not write JSON: {}", e.0),
        }
    }

    pub fn problema(&self, base_url: &str, instancia: &str) -> Problema {
        match self {
            Self::Falha(f) => {
                let p = f.problema();
                Problema {
                    tipo: tipo(base_url, p.path),
                    titulo: p.titulo.to_string(),
                    status: status(f.status()),
                    detalhe: f.mensagem.clone(),
                    instancia: instancia.to_string(),
                    com_timestamp: true,
                    erros: None,
                }
            }
            Self::CorpoAusente(_) | Self::Json(_) => sistema(
                base_url,
                StatusCode::BAD_REQUEST,
                Some(self.mensagem()),
                instancia,
            ),
            Self::CamposInvalidos(_) => Problema {
                titulo: TITULO_CAMPOS_INVALIDOS.into(),
                ..sistema(
                    base_url,
                    StatusCode::BAD_REQUEST,
                    Some(self.mensagem()),
                    instancia,
                )
            },
            Self::ParametroAusente(nome) => Problema::padrao(
                StatusCode::BAD_REQUEST,
                format!("Required parameter '{nome}' is not present."),
                instancia,
            ),
            Self::ValidacaoMetodo => {
                Problema::padrao(StatusCode::BAD_REQUEST, MSG_VALIDACAO_METODO, instancia)
            }
            Self::TipoInvalido { .. } => Problema {
                tipo: tipo(base_url, PATH_TIPO_ARGUMENTO),
                titulo: TITULO_TIPO_ARGUMENTO.into(),
                status: StatusCode::BAD_REQUEST,
                detalhe: Some(self.mensagem()),
                instancia: instancia.to_string(),
                com_timestamp: true,
                erros: None,
            },
            Self::Observabilidade(erros) => Problema {
                tipo: "about:blank".into(),
                titulo: TITULO_OBSERVABILIDADE.into(),
                status: StatusCode::UNPROCESSABLE_ENTITY,
                detalhe: Some(format!("O cálculo contém {} erro(s).", erros.len())),
                instancia: instancia.to_string(),
                com_timestamp: true,
                // erroDetalhe só tem strings: a escrita não falha
                erros: lista_erros(erros).ok(),
            },
            Self::Escrita(_) => Problema::padrao(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to write request",
                instancia,
            ),
        }
    }
}
