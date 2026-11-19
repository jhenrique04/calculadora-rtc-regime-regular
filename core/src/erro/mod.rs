//! erros de negócio com a mesma taxonomia das exceções Java: cada [`Classe`]
//! corresponde a uma classe de exceção e define o status HTTP (pela
//! hierarquia) e o `ProblemType` usado no corpo `application/problem+json`

mod tabela;

use std::fmt;

pub use tabela::Classe;

/// família da exceção Java, que decide o handler (e o status) no Spring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Categoria {
    Validacao,
    BadRequest,
    NaoEncontrada,
    Interno,
}

impl Categoria {
    pub fn status(self) -> u16 {
        match self {
            Self::Validacao => 422,
            Self::BadRequest => 400,
            Self::NaoEncontrada => 404,
            Self::Interno => 500,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoItem {
    Calculado,
    NaoImplementado,
    InconsistenciaEntrada,
    FalhaTecnica,
    CalculoSimulado,
}

impl EstadoItem {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Calculado => "CALCULADO",
            Self::NaoImplementado => "NAO_IMPLEMENTADO",
            Self::InconsistenciaEntrada => "INCONSISTENCIA_ENTRADA",
            Self::FalhaTecnica => "FALHA_TECNICA",
            Self::CalculoSimulado => "CALCULO_SIMULADO",
        }
    }
}

#[derive(Debug)]
pub struct CodigoErro {
    pub codigo: &'static str,
    pub campo: Option<&'static str>,
    pub estado: EstadoItem,
}

#[derive(Debug)]
pub struct Problema {
    pub titulo: &'static str,
    pub path: &'static str,
    pub codigo: Option<CodigoErro>,
    pub descricao: Option<&'static str>,
}

impl Problema {
    pub fn por_codigo(codigo: &str) -> Option<&'static Problema> {
        tabela::TODOS
            .iter()
            .copied()
            .find(|p| p.codigo.as_ref().is_some_and(|c| c.codigo == codigo))
    }
}

/// equivalente a uma exceção lançada pelos serviços Java
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Falha {
    pub classe: Classe,
    pub mensagem: Option<String>,
}

impl Falha {
    pub fn new(classe: Classe, mensagem: impl Into<String>) -> Self {
        Self {
            classe,
            mensagem: Some(mensagem.into()),
        }
    }

    /// `RuntimeException` genérica (cai no `ERRO_SISTEMA`, 500)
    pub fn sistema(mensagem: impl Into<String>) -> Self {
        Self::new(Classe::Sistema, mensagem)
    }

    /// exceção sem mensagem, como o `NullPointerException` lançado pelo `Map.of`
    pub fn sem_mensagem() -> Self {
        Self {
            classe: Classe::Sistema,
            mensagem: None,
        }
    }

    pub fn status(&self) -> u16 {
        self.classe.categoria().status()
    }

    pub fn problema(&self) -> &'static Problema {
        self.classe.problema()
    }
}

impl fmt::Display for Falha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mensagem.as_deref().unwrap_or("null"))
    }
}

impl std::error::Error for Falha {}

impl From<rusqlite::Error> for Falha {
    fn from(e: rusqlite::Error) -> Self {
        Self::sistema(e.to_string())
    }
}

pub type Resultado<T> = Result<T, Falha>;
