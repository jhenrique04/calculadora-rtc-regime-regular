//! acesso somente-leitura à base SQLite da calculadora. As consultas
//! reproduzem o SQL que o Hibernate gera para os repositórios Java, para que
//! ordem de linhas e critérios de vigência sejam idênticos

mod aliquota;
mod classificacao;
mod dados_abertos;
mod localidade;
mod nfse;
mod nomenclatura;
mod tratamento;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::NaiveDate;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, Params, Row};

use crate::decimal::BigDecimal;
use crate::erro::{Falha, Resultado};
use crate::expressao::java_double_to_string;

pub use aliquota::{AliquotaAdRem, AliquotaResultado, FormaAplicacao};
pub use classificacao::{
    ClassificacaoCalculo, ClassificacaoTributaria, Fundamentacao, SituacaoTributaria,
};
pub use dados_abertos::{
    CodigoDescricao, CreditosCbsIbs, CstClassificacao, FundamentacaoLegal, LinhaClassificacao,
    LinhaGrupoAtor, LinhaGrupoDfe, Municipio, SituacaoResumo, Uf, ValidadeDfe,
};
pub use localidade::{TipoDfe, VersaoBase, Vigencia};
pub use nfse::{CombinacaoIndicador, LinhaIndicador, SituacaoClassificacao};
pub use nomenclatura::Nomenclatura;
pub use tratamento::{TratamentoClassificacao, TratamentoTributario};

/// pool simples de conexões; SQLite em modo leitura aceita conexões
/// concorrentes, cada uma usada por uma thread por vez
pub struct Pool {
    caminho: PathBuf,
    livres: Mutex<Vec<Connection>>,
}

impl Pool {
    pub fn abrir(caminho: &Path) -> Resultado<Self> {
        let pool = Self {
            caminho: caminho.to_path_buf(),
            livres: Mutex::new(Vec::new()),
        };
        let base = pool.base()?;
        drop(base);
        Ok(pool)
    }

    fn conectar(&self) -> Resultado<Connection> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        Ok(Connection::open_with_flags(&self.caminho, flags)?)
    }

    pub fn base(&self) -> Resultado<Base<'_>> {
        let livre = match self.livres.lock() {
            Ok(mut v) => v.pop(),
            Err(e) => e.into_inner().pop(),
        };
        let conn = match livre {
            Some(c) => c,
            None => self.conectar()?,
        };
        Ok(Base {
            conn: Some(conn),
            pool: self,
        })
    }
}

/// conexão emprestada do [`Pool`]; volta para ele no drop
pub struct Base<'p> {
    conn: Option<Connection>,
    pool: &'p Pool,
}

impl Drop for Base<'_> {
    fn drop(&mut self) {
        if let Some(c) = self.conn.take() {
            match self.pool.livres.lock() {
                Ok(mut v) => v.push(c),
                Err(e) => e.into_inner().push(c),
            }
        }
    }
}

impl Base<'_> {
    fn conn(&self) -> Resultado<&Connection> {
        self.conn
            .as_ref()
            .ok_or_else(|| Falha::sistema("conexão indisponível"))
    }

    fn linhas<T>(
        &self,
        sql: &str,
        params: impl Params,
        f: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Resultado<Vec<T>> {
        let mut stmt = self.conn()?.prepare_cached(sql)?;
        let linhas = stmt.query_map(params, f)?;
        Ok(linhas.collect::<rusqlite::Result<Vec<T>>>()?)
    }

    /// `uniqueResult()` do Hibernate: nenhuma linha vira `None`, mais de uma
    /// é erro
    fn unica<T>(
        &self,
        sql: &str,
        params: impl Params,
        f: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Resultado<Option<T>> {
        let mut linhas = self.linhas(sql, params, f)?;
        if linhas.len() > 1 {
            return Err(Falha::new(
                crate::erro::Classe::IncorrectResultSizeDataAccess,
                format!(
                    "Query did not return a unique result: {} results were returned",
                    linhas.len()
                ),
            ));
        }
        Ok(linhas.pop())
    }

    fn existe(&self, sql: &str, params: impl Params) -> Resultado<bool> {
        let v = self.unica(sql, params, |r| r.get::<_, i64>(0))?;
        Ok(v.unwrap_or(0) != 0)
    }
}

/// `EntityNotFoundException` ao seguir um `@ManyToOne` órfão
pub(crate) fn entidade_ausente(entidade: &str, id: i64) -> Falha {
    Falha::sistema(format!(
        "Unable to find br.gov.serpro.rtc.domain.model.entity.{entidade} with id {id}"
    ))
}

pub(crate) fn data(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// `ResultSet.getBigDecimal` do sqlite-jdbc: REAL passa pela representação
/// textual do double
pub(crate) fn decimal(r: &Row<'_>, i: usize) -> rusqlite::Result<Option<BigDecimal>> {
    let texto = match r.get_ref(i)? {
        ValueRef::Null => return Ok(None),
        ValueRef::Integer(v) => return Ok(Some(BigDecimal::from_i64(v))),
        ValueRef::Real(v) => java_double_to_string(v),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(_) => {
            return Err(rusqlite::Error::InvalidColumnType(
                i,
                "blob".into(),
                rusqlite::types::Type::Blob,
            ));
        }
    };
    texto.parse().map(Some).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e))
    })
}

pub(crate) fn data_opcional(r: &Row<'_>, i: usize) -> rusqlite::Result<Option<NaiveDate>> {
    let Some(texto) = r.get::<_, Option<String>>(i)? else {
        return Ok(None);
    };
    NaiveDate::parse_from_str(&texto, "%Y-%m-%d")
        .map(Some)
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e))
        })
}

pub(crate) fn data_obrigatoria(r: &Row<'_>, i: usize) -> rusqlite::Result<NaiveDate> {
    data_opcional(r, i)?.ok_or(rusqlite::Error::InvalidColumnType(
        i,
        "null".into(),
        rusqlite::types::Type::Null,
    ))
}
