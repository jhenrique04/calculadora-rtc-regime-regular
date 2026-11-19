use chrono::NaiveDate;
use rusqlite::params;

use super::{Base, data};
use crate::erro::Resultado;

/// NCM e NBS têm tabelas espelhadas; só mudam nomes e prefixos de coluna
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nomenclatura {
    Ncm,
    Nbs,
}

struct Tabelas {
    base: &'static str,
    base_col: &'static str,
    aplicavel: &'static str,
    apl: &'static str,
    excecao: &'static str,
    exc: &'static str,
}

impl Nomenclatura {
    fn tabelas(self) -> Tabelas {
        match self {
            Self::Ncm => Tabelas {
                base: "ncm",
                base_col: "ncm",
                aplicavel: "ncm_aplicavel",
                apl: "ncma",
                excecao: "excecao_ncm_aplicavel",
                exc: "encm",
            },
            Self::Nbs => Tabelas {
                base: "nbs",
                base_col: "nbs",
                aplicavel: "nbs_aplicavel",
                apl: "nbsa",
                excecao: "excecao_nbs_aplicavel",
                exc: "enbs",
            },
        }
    }
}

impl Base<'_> {
    pub fn existe_codigo(&self, n: Nomenclatura, codigo: &str, dia: NaiveDate) -> Resultado<bool> {
        let t = n.tabelas();
        let d = data(dia);
        let sql = format!(
            "select case when exists(select 1 from {b} n1_0 where n1_0.{c}_cd=? and ? between n1_0.{c}_inicio_vigencia and coalesce(n1_0.{c}_fim_vigencia,?)) then 1 else 0 end",
            b = t.base,
            c = t.base_col
        );
        self.existe(&sql, params![codigo, d, d])
    }

    /// `tem`: a classificação possui anexo de NCM/NBS aplicáveis vigente
    pub fn classificacao_tem_aplicavel(
        &self,
        n: Nomenclatura,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<bool> {
        let t = n.tabelas();
        let d = data(dia);
        let sql = format!(
            "select case when exists(select 1 from {a} na1_0 where na1_0.{p}_cltr_id=? and ? between na1_0.{p}_inicio_vigencia and coalesce(na1_0.{p}_fim_vigencia,?)) then 1 else 0 end",
            a = t.aplicavel,
            p = t.apl
        );
        self.existe(&sql, params![id_classificacao, d, d])
    }

    pub fn tem_aplicavel(
        &self,
        n: Nomenclatura,
        codigo: &str,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<bool> {
        let t = n.tabelas();
        let d = data(dia);
        let sql = format!(
            "select case when exists(select 1 from {a} na1_0 where na1_0.{p}_{c}_cd=substr(?,1,length(na1_0.{p}_{c}_cd)) and na1_0.{p}_cltr_id=? and ? between na1_0.{p}_inicio_vigencia and coalesce(na1_0.{p}_fim_vigencia,?)) then 1 else 0 end",
            a = t.aplicavel,
            p = t.apl,
            c = t.base_col
        );
        self.existe(&sql, params![codigo, id_classificacao, d, d])
    }

    pub fn tem_excecao_aplicavel(
        &self,
        n: Nomenclatura,
        codigo: &str,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<bool> {
        let t = n.tabelas();
        let d = data(dia);
        let sql = format!(
            "select case when exists(select 1 from {e} ena1_0 join {a} na1_0 on na1_0.{p}_id=ena1_0.{x}_{p}_id where na1_0.{p}_{c}_cd=substr(?,1,length(na1_0.{p}_{c}_cd)) and ena1_0.{x}_{c}_cd=substr(?,1,length(ena1_0.{x}_{c}_cd)) and na1_0.{p}_cltr_id=? and ? between ena1_0.{x}_inicio_vigencia and coalesce(ena1_0.{x}_fim_vigencia,?)) then 1 else 0 end",
            e = t.excecao,
            a = t.aplicavel,
            p = t.apl,
            x = t.exc,
            c = t.base_col
        );
        self.existe(&sql, params![codigo, codigo, id_classificacao, d, d])
    }

    pub fn tem_aplicavel_sem_excecao(
        &self,
        n: Nomenclatura,
        codigo: &str,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<bool> {
        let t = n.tabelas();
        let d = data(dia);
        let sql = format!(
            "select case when exists(select 1 from {a} na1_0 left join {e} ena1_0 on ena1_0.{x}_{p}_id=na1_0.{p}_id where na1_0.{p}_{c}_cd=substr(?,1,length(na1_0.{p}_{c}_cd)) and na1_0.{p}_cltr_id=? and ? between na1_0.{p}_inicio_vigencia and coalesce(na1_0.{p}_fim_vigencia,?) and (ena1_0.{x}_id is null or ? not between ena1_0.{x}_inicio_vigencia and coalesce(ena1_0.{x}_fim_vigencia,?))) then 1 else 0 end",
            e = t.excecao,
            a = t.aplicavel,
            p = t.apl,
            x = t.exc,
            c = t.base_col
        );
        self.existe(&sql, params![codigo, id_classificacao, d, d, d, d])
    }
}
