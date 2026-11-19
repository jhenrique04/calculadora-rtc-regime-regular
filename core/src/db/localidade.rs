use chrono::NaiveDate;
use rusqlite::params;

use super::{Base, data, data_obrigatoria, data_opcional, decimal, entidade_ausente};
use crate::decimal::BigDecimal;
use crate::erro::Resultado;

#[derive(Debug, Clone)]
pub struct TipoDfe {
    pub id: i64,
    pub tipo: Option<i32>,
    pub sigla: String,
    pub descricao: String,
}

#[derive(Debug, Clone)]
pub struct VersaoBase {
    pub id: i64,
    pub data: Option<String>,
    pub descricao: Option<String>,
    pub numero: String,
}

#[derive(Debug, Clone)]
pub struct Vigencia<T> {
    pub id: i64,
    pub valor: T,
    pub inicio: NaiveDate,
    pub fim: Option<NaiveDate>,
}

fn linha_tipo_dfe(r: &rusqlite::Row<'_>) -> rusqlite::Result<TipoDfe> {
    Ok(TipoDfe {
        id: r.get(0)?,
        descricao: r.get(1)?,
        sigla: r.get(4)?,
        tipo: r.get(5)?,
    })
}

impl Base<'_> {
    pub fn codigo_uf(&self, sigla: &str) -> Resultado<Option<i64>> {
        self.unica(
            "select u1_0.uf_cd from uf u1_0 where u1_0.uf_sigla=upper(?)",
            [sigla],
            |r| r.get(0),
        )
    }

    pub fn existe_uf(&self, sigla: &str) -> Resultado<bool> {
        self.existe(
            "select case when exists(select 1 from uf u1_0 where u1_0.uf_sigla=upper(?)) then 1 else 0 end",
            [sigla],
        )
    }

    /// 0 = ok, 1 = município inexistente, 2 = município de outra UF
    pub fn validar_municipio(&self, codigo: i64, sigla_uf: &str) -> Resultado<i64> {
        let v = self.unica(
            "select case when count(m1_0.muni_cd)=0 then 1 when count(m1_0.muni_cd)>0 and lower(u1_0.uf_sigla)<>lower(?) then 2 else 0 end from municipio m1_0 join uf u1_0 on u1_0.uf_cd=m1_0.muni_uf_cd where m1_0.muni_cd=?",
            params![sigla_uf, codigo],
            |r| r.get::<_, Option<i64>>(0),
        )?;
        Ok(v.flatten().unwrap_or(0))
    }

    /// `None` quando o município não existe; `Some(None)` se ele não tiver UF
    pub fn sigla_uf_do_municipio(&self, codigo: i64) -> Resultado<Option<Option<String>>> {
        self.unica(
            "select u1_0.uf_sigla from municipio m1_0 left join uf u1_0 on u1_0.uf_cd=m1_0.muni_uf_cd where m1_0.muni_cd=?",
            [codigo],
            |r| r.get::<_, Option<String>>(0),
        )
    }

    pub fn tipo_dfe(&self, id: i64) -> Resultado<TipoDfe> {
        self.unica(
            "select td1_0.tpdf_id,td1_0.tpdf_descricao,td1_0.tpdf_fim_vigencia,td1_0.tpdf_inicio_vigencia,td1_0.tpdf_sigla,td1_0.tpdf_tipo from tipo_dfe td1_0 where td1_0.tpdf_id=?",
            [id],
            linha_tipo_dfe,
        )?
        .ok_or_else(|| entidade_ausente("TipoDfe", id))
    }

    pub fn tipo_dfe_por_tipo(&self, tipo: i32, dia: NaiveDate) -> Resultado<Option<TipoDfe>> {
        let d = data(dia);
        self.unica(
            "select td1_0.tpdf_id,td1_0.tpdf_descricao,td1_0.tpdf_fim_vigencia,td1_0.tpdf_inicio_vigencia,td1_0.tpdf_sigla,td1_0.tpdf_tipo from tipo_dfe td1_0 where td1_0.tpdf_tipo=? and ? between td1_0.tpdf_inicio_vigencia and coalesce(td1_0.tpdf_fim_vigencia,?)",
            params![tipo, d, d],
            linha_tipo_dfe,
        )
    }

    pub fn ultima_versao(&self) -> Resultado<Option<VersaoBase>> {
        self.unica(
            "select vbd1_0.vrbd_id,vbd1_0.vrbd_data,vbd1_0.vrbd_descricao,vbd1_0.vrbd_versao_base_dado from versao_base_dado vbd1_0 order by vbd1_0.vrbd_id desc limit ?",
            [1],
            |r| {
                Ok(VersaoBase {
                    id: r.get(0)?,
                    data: r.get(1)?,
                    descricao: r.get(2)?,
                    numero: r.get(3)?,
                })
            },
        )
    }

    pub fn valor_redutor_compra_gov(&self, dia: NaiveDate) -> Resultado<Option<BigDecimal>> {
        let d = data(dia);
        let v = self.unica(
            "select rcg1_0.rcgo_valor from redutor_compra_governamental rcg1_0 where rcg1_0.rcgo_inicio_vigencia<=? and (rcg1_0.rcgo_fim_vigencia is null or rcg1_0.rcgo_fim_vigencia>=?)",
            [&d, &d],
            |r| decimal(r, 0),
        )?;
        Ok(v.flatten())
    }

    pub fn percentual_transferencia_cbs(&self, dia: NaiveDate) -> Resultado<Option<BigDecimal>> {
        let d = data(dia);
        let v = self.unica(
            "select tc1_0.tceg_valor from transferencia_cbs_ente_gov tc1_0 where tc1_0.tceg_inicio_vigencia<=? and (tc1_0.tceg_fim_vigencia is null or tc1_0.tceg_fim_vigencia>=?)",
            [&d, &d],
            |r| decimal(r, 0),
        )?;
        Ok(v.flatten())
    }

    pub fn redutores_compra_gov(&self) -> Resultado<Vec<Vigencia<BigDecimal>>> {
        self.vigencias(
            "select rcg1_0.rcgo_id,rcg1_0.rcgo_fim_vigencia,rcg1_0.rcgo_inicio_vigencia,rcg1_0.rcgo_valor from redutor_compra_governamental rcg1_0 order by rcg1_0.rcgo_inicio_vigencia",
        )
    }

    pub fn transferencias_cbs(&self) -> Resultado<Vec<Vigencia<BigDecimal>>> {
        self.vigencias(
            "select tc1_0.tceg_id,tc1_0.tceg_fim_vigencia,tc1_0.tceg_inicio_vigencia,tc1_0.tceg_valor from transferencia_cbs_ente_gov tc1_0 order by tc1_0.tceg_inicio_vigencia",
        )
    }

    fn vigencias(&self, sql: &str) -> Resultado<Vec<Vigencia<BigDecimal>>> {
        let linhas = self.linhas(sql, [], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                data_opcional(r, 1)?,
                data_obrigatoria(r, 2)?,
                decimal(r, 3)?,
            ))
        })?;
        Ok(linhas
            .into_iter()
            .map(|(id, fim, inicio, valor)| Vigencia {
                id,
                valor: valor.unwrap_or_else(BigDecimal::zero),
                inicio,
                fim,
            })
            .collect())
    }
}
