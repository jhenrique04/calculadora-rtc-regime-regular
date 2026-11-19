use chrono::NaiveDate;
use rusqlite::params;

use super::{Base, TipoDfe, data};
use crate::erro::Resultado;

#[derive(Debug, Clone)]
pub struct ClassificacaoTributaria {
    pub id: i64,
    pub codigo: String,
    pub nomenclatura: Option<String>,
    pub cst: String,
}

#[derive(Debug, Clone)]
pub struct ClassificacaoCalculo {
    pub id: i64,
    pub codigo: String,
    pub tipo_aliquota: String,
    pub mono_padrao: bool,
    pub mono_reten: bool,
    pub mono_ret: bool,
    pub mono_diferimento: bool,
    pub grupo_reducao: bool,
}

#[derive(Debug, Clone)]
pub struct SituacaoTributaria {
    pub id: i64,
    pub codigo: String,
    pub descricao: String,
    pub grupo_ibs_cbs: bool,
    pub grupo_ibs_cbs_mono: bool,
    pub grupo_reducao: bool,
    pub grupo_diferimento: bool,
    pub grupo_transf_cred: bool,
    pub grupo_cred_pres_ibs_zfm: bool,
    pub grupo_ajuste_compet: bool,
}

#[derive(Debug, Clone)]
pub struct Fundamentacao {
    pub texto_curto: Option<String>,
    pub memoria_calculo: Option<String>,
}

impl Base<'_> {
    pub fn classificacao_tributaria(
        &self,
        codigo: &str,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<ClassificacaoTributaria>> {
        let d = data(dia);
        self.unica(
            "SELECT ct.CLTR_ID as id, ct.CLTR_CD as codigo, ct.CLTR_NOMENCLATURA as nomenclatura, st.SITR_CD as cst
            FROM CLASSIFICACAO_TRIBUTARIA ct
            JOIN SITUACAO_TRIBUTARIA st ON ct.CLTR_SITR_ID = st.SITR_ID
            JOIN TRIBUTO_SITUACAO_TRIBUTARIA tst ON tst.TRST_SITR_ID = st.SITR_ID
            JOIN TRIBUTO tb ON tst.TRST_TBTO_ID = tb.TBTO_ID
            WHERE ct.CLTR_CD = ?1
            AND tb.TBTO_ID = ?2
            AND ?3 BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, ?3)
            AND ?3 BETWEEN st.SITR_INICIO_VIGENCIA AND COALESCE(st.SITR_FIM_VIGENCIA, ?3)
            AND ?3 BETWEEN tst.TRST_INICIO_VIGENCIA AND COALESCE(tst.TRST_FIM_VIGENCIA, ?3)
            AND ?3 BETWEEN tb.TBTO_INICIO_VIGENCIA AND COALESCE(tb.TBTO_FIM_VIGENCIA, ?3)",
            params![codigo, id_tributo, d],
            |r| {
                Ok(ClassificacaoTributaria {
                    id: r.get(0)?,
                    codigo: r.get(1)?,
                    nomenclatura: r.get(2)?,
                    cst: r.get(3)?,
                })
            },
        )
    }

    pub fn classificacao_calculo(&self, id: i64) -> Resultado<Option<ClassificacaoCalculo>> {
        self.unica(
            "select ct1_0.cltr_id,ct1_0.cltr_cd,ct1_0.cltr_tipo_aliquota,ct1_0.cltr_ind_gmonopadrao,ct1_0.cltr_ind_gmonoreten,ct1_0.cltr_ind_gmonoret,ct1_0.cltr_ind_gmonodif,st1_0.sitr_ind_gred from classificacao_tributaria ct1_0 join situacao_tributaria st1_0 on st1_0.sitr_id=ct1_0.cltr_sitr_id where ct1_0.cltr_id=?",
            [id],
            |r| {
                Ok(ClassificacaoCalculo {
                    id: r.get(0)?,
                    codigo: r.get(1)?,
                    tipo_aliquota: r.get(2)?,
                    mono_padrao: r.get(3)?,
                    mono_reten: r.get(4)?,
                    mono_ret: r.get(5)?,
                    mono_diferimento: r.get(6)?,
                    grupo_reducao: r.get(7)?,
                })
            },
        )
    }

    pub fn existe_cst(&self, cst: &str, id_tributo: i64, dia: NaiveDate) -> Resultado<bool> {
        let d = data(dia);
        self.existe(
            "select case when exists(select 1 from tributo_situacao_tributaria tst1_0 join situacao_tributaria st1_0 on st1_0.sitr_id=tst1_0.trst_sitr_id where tst1_0.trst_tbto_id=? and st1_0.sitr_cd=? and ? between tst1_0.trst_inicio_vigencia and coalesce(tst1_0.trst_fim_vigencia,?) and ? between st1_0.sitr_inicio_vigencia and coalesce(st1_0.sitr_fim_vigencia,?)) then 1 else 0 end",
            params![id_tributo, cst, d, d, d, d],
        )
    }

    pub fn situacao_tributaria_por_codigo(
        &self,
        cst: &str,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<SituacaoTributaria>> {
        let d = data(dia);
        self.unica(
            "SELECT s.SITR_ID, s.SITR_CD, s.SITR_DESCRICAO, s.SITR_IND_GIBSCBS, s.SITR_IND_GIBSCBSMONO, s.SITR_IND_GRED,
                s.SITR_IND_GDIF, s.SITR_IND_GTRANSFCRED, s.SITR_IND_GCREDPRESIBSZFM, s.SITR_IND_GAJUSTECOMPET
            FROM SITUACAO_TRIBUTARIA s
            JOIN TRIBUTO_SITUACAO_TRIBUTARIA tst ON tst.TRST_SITR_ID = s.SITR_ID
            JOIN TRIBUTO t ON t.TBTO_ID = tst.TRST_TBTO_ID
            WHERE COALESCE(s.SITR_CD, '') = COALESCE(?1, '')
              AND COALESCE(t.TBTO_ID, 0) = COALESCE(?2, 0)
              AND COALESCE(?3, '') BETWEEN COALESCE(tst.TRST_INICIO_VIGENCIA, '') AND COALESCE(tst.TRST_FIM_VIGENCIA, ?3)
              AND COALESCE(?3, '') BETWEEN COALESCE(s.SITR_INICIO_VIGENCIA, '') AND COALESCE(s.SITR_FIM_VIGENCIA, ?3)
              AND COALESCE(?3, '') BETWEEN COALESCE(t.TBTO_INICIO_VIGENCIA, '') AND COALESCE(t.TBTO_FIM_VIGENCIA, ?3)
            LIMIT 1",
            params![cst, id_tributo, d],
            |r| {
                Ok(SituacaoTributaria {
                    id: r.get(0)?,
                    codigo: r.get(1)?,
                    descricao: r.get(2)?,
                    grupo_ibs_cbs: r.get(3)?,
                    grupo_ibs_cbs_mono: r.get(4)?,
                    grupo_reducao: r.get(5)?,
                    grupo_diferimento: r.get(6)?,
                    grupo_transf_cred: r.get(7)?,
                    grupo_cred_pres_ibs_zfm: r.get(8)?,
                    grupo_ajuste_compet: r.get(9)?,
                })
            },
        )
    }

    /// tipos de DFe vinculados à classificação (`TipoDfeClassificacaoRepository.buscar`).
    /// Mesmo SQL sem `order by` do Hibernate e carga do `@ManyToOne` por id, para
    /// manter a ordem que o plano do SQLite produz
    pub fn tipos_dfe_da_classificacao(
        &self,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<Vec<TipoDfe>> {
        let d = data(dia);
        let ids = self.linhas(
            "select tdc1_0.tdcl_id,tdc1_0.tdcl_cltr_id,tdc1_0.tdcl_fim_vigencia,tdc1_0.tdcl_inicio_vigencia,tdc1_0.tdcl_tpdf_id from tipo_dfe_classificacao tdc1_0 where tdc1_0.tdcl_cltr_id=? and (tdc1_0.tdcl_inicio_vigencia<=? and (tdc1_0.tdcl_fim_vigencia is null or tdc1_0.tdcl_fim_vigencia>=?))",
            params![id_classificacao, d, d],
            |r| r.get::<_, i64>(4),
        )?;
        ids.into_iter().map(|id| self.tipo_dfe(id)).collect()
    }

    pub fn fundamentacao(
        &self,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<Fundamentacao>> {
        let d = data(dia);
        self.unica(
            "select fl1_0.fdlg_texto_curto,ct1_0.cltr_memoria_calculo from fundamentacao_classificacao fc1_0 join fundamentacao_legal fl1_0 on fl1_0.fdlg_id=fc1_0.fdcl_fdlg_id join classificacao_tributaria ct1_0 on ct1_0.cltr_id=fc1_0.fdcl_cltr_id where fc1_0.fdcl_cltr_id=? and ? between fc1_0.fdcl_inicio_vigencia and coalesce(fc1_0.fdcl_fim_vigencia,?)",
            params![id_classificacao, d, d],
            |r| {
                Ok(Fundamentacao {
                    texto_curto: r.get(0)?,
                    memoria_calculo: r.get(1)?,
                })
            },
        )
    }
}
