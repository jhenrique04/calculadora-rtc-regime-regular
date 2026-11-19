//! `CalculoTributoService`: calcula IS, CBS e IBS e consolida o grupo `IBSCBS`

use chrono::NaiveDate;

use super::Servicos;
use super::cbs_ibs::{
    CbsIbsSaida, DiferimentoMonofasiaSaida, EtapaMonofasiaSaida, MonofasiaSaida, ParametrosCbsIbs,
    Tributo,
};
use super::imposto_seletivo::AliquotaImpostoSeletivo;
use crate::db::TratamentoClassificacao;
use crate::decimal::BigDecimal;
use crate::erro::{Falha, Resultado};
use crate::modelo::entrada::{Item, TipoEnteGov};
use crate::modelo::roc::{
    Cbs, EtapaMonofasia, GrupoIbsCbs, GrupoTributo, IbsCbs, IbsMun, IbsUf, ImpostoSeletivo,
    Monofasia, MonofasiaDiferimento, MonofasiaPadrao, MonofasiaRetencao, MonofasiaRetido,
    ReducaoAliquota, TributacaoCompraGov, TributacaoRegular, Tributos,
};
use crate::util::arredondamento::dividir_por_cem;

const ANO_INICIO_COMPRA_GOV_EFETIVA: i32 = 2027;

#[derive(Debug, Clone)]
pub struct Tratamentos {
    pub cbs_ibs: TratamentoClassificacao,
    pub imposto_seletivo: Option<TratamentoClassificacao>,
    pub desoneracao: Option<TratamentoClassificacao>,
    pub aliquota_is: Option<AliquotaImpostoSeletivo>,
    pub tem_desoneracao: bool,
}

pub struct OperacaoCalculo<'a> {
    pub dia: NaiveDate,
    pub codigo_municipio: Option<i64>,
    pub codigo_uf: i64,
    pub tp_ente: Option<TipoEnteGov>,
    pub p_redutor: BigDecimal,
    pub item: &'a Item,
    pub quantidade: &'a BigDecimal,
    pub tratamentos: Tratamentos,
}

fn zero() -> BigDecimal {
    BigDecimal::zero()
}

fn ou_zero(v: Option<&BigDecimal>) -> BigDecimal {
    v.cloned().unwrap_or_default()
}

impl Servicos<'_> {
    pub fn calcular_tributos(&self, op: &OperacaoCalculo<'_>) -> Resultado<Tributos> {
        let t = &op.tratamentos;
        let tratamento_memoria = match (&t.desoneracao, t.tem_desoneracao) {
            (Some(d), true) => d,
            _ => &t.cbs_ibs,
        };
        let is = match (&t.imposto_seletivo, &op.item.imposto_seletivo) {
            (Some(tratamento), Some(entrada)) => {
                let aliquota = t.aliquota_is.clone().unwrap_or_default();
                let mut is = self.calcular_imposto_seletivo(entrada, tratamento, &aliquota)?;
                self.memoria_imposto_seletivo(tratamento, &mut is, op.dia)?;
                Some(is)
            }
            _ => None,
        };
        let is_calculado = is.as_ref().map_or_else(zero, ImpostoSeletivo::valor);
        let calcular = |tributo, codigo_uf, codigo_municipio| -> Resultado<Option<CbsIbsSaida>> {
            let p = ParametrosCbsIbs {
                tributo,
                codigo_uf,
                codigo_municipio,
                item: op.item,
                quantidade: op.quantidade,
                tratamento: &t.cbs_ibs,
                is_calculado: &is_calculado,
                tem_tributacao_regular: t.tem_desoneracao,
                dia: op.dia,
                tp_ente: op.tp_ente,
                p_redutor: &op.p_redutor,
            };
            let mut saida = self.calcular_cbs_ibs(&p)?;
            if let Some(s) = saida.as_mut() {
                self.memoria_cbs_ibs(
                    tratamento_memoria,
                    s,
                    op.quantidade,
                    op.item.unidade.as_deref(),
                    op.dia,
                )?;
            }
            Ok(saida)
        };
        // o Java calcula em paralelo; a precedência de erro do allOf é CBS, UF, município
        let mut cbs = calcular(Tributo::Cbs, None, None)?;
        let mut ibs_uf = calcular(Tributo::IbsEstadual, Some(op.codigo_uf), None)?;
        let mut ibs_mun = calcular(Tributo::IbsMunicipal, None, op.codigo_municipio)?;
        let ibs_cbs = self.ibs_cbs(op, &mut cbs, &mut ibs_uf, &mut ibs_mun)?;
        Ok(Tributos {
            is,
            ibs_cbs: Some(ibs_cbs),
        })
    }

    fn ibs_cbs(
        &self,
        op: &OperacaoCalculo<'_>,
        cbs: &mut Option<CbsIbsSaida>,
        ibs_uf: &mut Option<CbsIbsSaida>,
        ibs_mun: &mut Option<CbsIbsSaida>,
    ) -> Resultado<IbsCbs> {
        let monofasia = monofasia(cbs.as_ref(), ibs_uf.as_ref());
        let g_ibs_cbs = if monofasia.is_none() {
            self.grupo_ibs_cbs(op, cbs, ibs_uf, ibs_mun)?
        } else {
            None
        };
        Ok(IbsCbs {
            cst: op.item.cst.clone(),
            c_class_trib: op.item.c_class_trib.clone(),
            g_ibs_cbs,
            g_ibs_cbs_mono: monofasia,
            ..IbsCbs::default()
        })
    }

    fn grupo_ibs_cbs(
        &self,
        op: &OperacaoCalculo<'_>,
        cbs: &mut Option<CbsIbsSaida>,
        ibs_uf: &mut Option<CbsIbsSaida>,
        ibs_mun: &mut Option<CbsIbsSaida>,
    ) -> Resultado<Option<GrupoIbsCbs>> {
        if cbs.is_none() && ibs_uf.is_none() && ibs_mun.is_none() {
            return Ok(None);
        }
        let p_transferencia = self.percentual_transferencia_cbs(op.dia)?;
        let mut compra_gov = compra_governamental(cbs.as_ref(), ibs_uf.as_ref(), ibs_mun.as_ref())?;
        let efetiva = match &compra_gov {
            Some(c) => valores_efetivos(c, op.tp_ente, &p_transferencia, op.dia)?,
            None => None,
        };
        if let Some(ef) = &efetiva {
            impacto_compra_gov(cbs.as_mut(), &ef.p_aliq_cbs, &ef.v_trib_cbs);
            impacto_compra_gov(ibs_uf.as_mut(), &ef.p_aliq_ibs_uf, &ef.v_trib_ibs_uf);
            impacto_compra_gov(ibs_mun.as_mut(), &ef.p_aliq_ibs_mun, &ef.v_trib_ibs_mun);
        }
        let v_bc = [cbs.as_ref(), ibs_uf.as_ref(), ibs_mun.as_ref()]
            .into_iter()
            .flatten()
            .find_map(|s| s.base_calculo.clone());
        let g_ibs_uf = ibs_uf.as_ref().map(|s| IbsUf(grupo_tributo(s)));
        let g_ibs_mun = ibs_mun.as_ref().map(|s| IbsMun(grupo_tributo(s)));
        let v_uf = g_ibs_uf
            .as_ref()
            .map_or_else(zero, |g| ou_zero(g.0.valor.as_ref()));
        let v_mun = g_ibs_mun
            .as_ref()
            .map_or_else(zero, |g| ou_zero(g.0.valor.as_ref()));
        let g_cbs = cbs.as_ref().map(|s| Cbs(grupo_tributo(s)));
        let tributacao_regular =
            tributacao_regular(cbs.as_ref(), ibs_uf.as_ref(), ibs_mun.as_ref());
        if tributacao_regular.is_some()
            && let Some(c) = compra_gov.as_mut()
        {
            c.zerar_valores();
        }
        Ok(Some(GrupoIbsCbs {
            v_bc,
            g_ibs_uf,
            g_ibs_mun,
            v_ibs: Some(v_uf.add(&v_mun)),
            g_cbs,
            g_trib_regular: tributacao_regular,
            g_trib_compra_gov: compra_gov,
        }))
    }
}

fn grupo_tributo(s: &CbsIbsSaida) -> GrupoTributo {
    GrupoTributo {
        aliquota: s.aliquota.clone(),
        g_dif: s.grupo_diferimento.clone(),
        g_dev_trib: None,
        g_red: s.grupo_reducao.clone(),
        valor: s.tributo_devido.clone(),
        memoria_calculo: s.memoria_calculo.clone(),
    }
}

fn impacto_compra_gov(
    saida: Option<&mut CbsIbsSaida>,
    aliquota: &Option<BigDecimal>,
    valor: &Option<BigDecimal>,
) {
    let Some(s) = saida else { return };
    let reducao = s.grupo_reducao.get_or_insert_with(|| ReducaoAliquota {
        p_red_aliq: Some(zero()),
        p_aliq_efet: None,
    });
    match s.tributacao_regular.as_mut() {
        Some(tr) => {
            reducao.p_red_aliq = Some(zero());
            reducao.p_aliq_efet = Some(zero());
            s.tributo_devido = Some(zero());
            tr.aliquota_efetiva = aliquota.clone();
            tr.tributo_devido = valor.clone();
        }
        None => {
            reducao.p_aliq_efet = aliquota.clone();
            s.tributo_devido = valor.clone();
        }
    }
}

/// `getTributacaoRegular`: cada etapa devolve nulo se o tributo não tiver o grupo
fn tributacao_regular(
    cbs: Option<&CbsIbsSaida>,
    ibs_uf: Option<&CbsIbsSaida>,
    ibs_mun: Option<&CbsIbsSaida>,
) -> Option<TributacaoRegular> {
    type Campos = fn(&mut TributacaoRegular) -> (&mut Option<BigDecimal>, &mut Option<BigDecimal>);
    let etapa = |s: Option<&CbsIbsSaida>, acc: Option<TributacaoRegular>, campos: Campos| {
        let tr = s?.tributacao_regular.as_ref()?;
        let mut b = acc.unwrap_or_else(|| TributacaoRegular {
            cst_reg: tr.cst.clone(),
            c_class_trib_reg: tr.c_class_trib.clone(),
            ..TributacaoRegular::default()
        });
        let (p, v) = campos(&mut b);
        *p = tr.aliquota_efetiva.clone();
        *v = tr.tributo_devido.clone();
        Some(b)
    };
    let b = etapa(cbs, None, |b| {
        (&mut b.p_aliq_efet_reg_cbs, &mut b.v_trib_reg_cbs)
    });
    let b = etapa(ibs_uf, b, |b| {
        (&mut b.p_aliq_efet_reg_ibs_uf, &mut b.v_trib_reg_ibs_uf)
    });
    etapa(ibs_mun, b, |b| {
        (&mut b.p_aliq_efet_reg_ibs_mun, &mut b.v_trib_reg_ibs_mun)
    })
}

fn compra_governamental(
    cbs: Option<&CbsIbsSaida>,
    ibs_uf: Option<&CbsIbsSaida>,
    ibs_mun: Option<&CbsIbsSaida>,
) -> Resultado<Option<TributacaoCompraGov>> {
    let grupo = |s: Option<&CbsIbsSaida>| s.and_then(|s| s.compra_gov.clone());
    match (grupo(cbs), grupo(ibs_uf), grupo(ibs_mun)) {
        (Some(c), Some(uf), Some(mun)) => Ok(Some(TributacaoCompraGov {
            p_aliq_cbs: c.p_aliq_cbs,
            v_trib_cbs: c.v_trib_cbs,
            p_aliq_ibs_uf: uf.p_aliq_ibs_uf,
            v_trib_ibs_uf: uf.v_trib_ibs_uf,
            p_aliq_ibs_mun: mun.p_aliq_ibs_mun,
            v_trib_ibs_mun: mun.v_trib_ibs_mun,
        })),
        (None, None, None) => Ok(None),
        _ => Err(Falha::sistema(
            "Erro ao consolidar valores de compras governamentais. Algum dos tributos não calculou compra governamental.",
        )),
    }
}

fn soma3(
    a: &Option<BigDecimal>,
    b: &Option<BigDecimal>,
    c: &Option<BigDecimal>,
) -> Resultado<BigDecimal> {
    match (a, b, c) {
        (Some(a), Some(b), Some(c)) => Ok(a.add(b).add(c)),
        _ => Err(Falha::sistema(
            "Cannot read field \"scale\" because \"augend\" is null",
        )),
    }
}

fn valores_efetivos(
    c: &TributacaoCompraGov,
    tp_ente: Option<TipoEnteGov>,
    p_transferencia: &BigDecimal,
    dia: NaiveDate,
) -> Resultado<Option<TributacaoCompraGov>> {
    let Some(ente) = tp_ente else { return Ok(None) };
    if dia < NaiveDate::from_ymd_opt(ANO_INICIO_COMPRA_GOV_EFETIVA, 1, 1).unwrap_or(NaiveDate::MAX)
    {
        return Ok(Some(c.clone()));
    }
    if ente.eh_uniao() {
        return Ok(Some(TributacaoCompraGov {
            p_aliq_cbs: Some(soma3(&c.p_aliq_cbs, &c.p_aliq_ibs_uf, &c.p_aliq_ibs_mun)?),
            v_trib_cbs: Some(soma3(&c.v_trib_cbs, &c.v_trib_ibs_uf, &c.v_trib_ibs_mun)?),
            ..TributacaoCompraGov::default()
        }));
    }
    let (Some(p_cbs), Some(v_cbs)) = (&c.p_aliq_cbs, &c.v_trib_cbs) else {
        return Err(Falha::sistema(
            "Cannot invoke \"java.math.BigDecimal.multiply(java.math.BigDecimal)\" because \"this.pAliqCBS\" is null",
        ));
    };
    let fracao = dividir_por_cem(p_transferencia);
    let p_transferir = p_cbs.multiply(&fracao);
    let v_transferir = v_cbs.multiply(&fracao);
    let mut ef = TributacaoCompraGov {
        p_aliq_cbs: Some(p_cbs.subtract(&p_transferir)),
        v_trib_cbs: Some(v_cbs.subtract(&v_transferir)),
        ..TributacaoCompraGov::default()
    };
    let p_final = soma3(&Some(p_transferir), &c.p_aliq_ibs_uf, &c.p_aliq_ibs_mun)?;
    let v_final = soma3(&Some(v_transferir), &c.v_trib_ibs_uf, &c.v_trib_ibs_mun)?;
    if ente.eh_estado() {
        ef.p_aliq_ibs_uf = Some(p_final);
        ef.v_trib_ibs_uf = Some(v_final);
    } else if ente.eh_municipio() {
        ef.p_aliq_ibs_mun = Some(p_final);
        ef.v_trib_ibs_mun = Some(v_final);
    }
    Ok(Some(ef))
}

/// `GrupoEtapaMonofasiaOutput.merge` com o lado municipal sempre vazio: devolve o estadual
fn etapa_ibs(
    ibs_uf: Option<&CbsIbsSaida>,
    f: fn(&MonofasiaSaida) -> Option<&EtapaMonofasiaSaida>,
) -> Option<&EtapaMonofasiaSaida> {
    ibs_uf.and_then(|s| s.grupo_monofasia.as_ref()).and_then(f)
}

fn etapa_cbs(
    cbs: Option<&CbsIbsSaida>,
    f: fn(&MonofasiaSaida) -> Option<&EtapaMonofasiaSaida>,
) -> Option<&EtapaMonofasiaSaida> {
    cbs.and_then(|s| s.grupo_monofasia.as_ref()).and_then(f)
}

/// builders de `Monofasia*Domain.create`: o grupo só existe se o IBS estadual tiver a etapa
fn etapa_monofasia(
    cbs: Option<&CbsIbsSaida>,
    ibs_uf: Option<&CbsIbsSaida>,
    f: fn(&MonofasiaSaida) -> Option<&EtapaMonofasiaSaida>,
) -> Option<EtapaMonofasia> {
    let ibs = etapa_ibs(ibs_uf, f)?;
    let mut e = EtapaMonofasia::default();
    if let Some(c) = etapa_cbs(cbs, f) {
        e.q_bc = c.quantidade.clone();
        e.ad_rem_cbs = c.aliquota_ad_rem.clone();
        e.v_cbs = c.valor.clone();
    }
    e.q_bc = ibs.quantidade.clone();
    e.ad_rem_ibs = ibs.aliquota_ad_rem.clone();
    e.v_ibs = ibs.valor.clone();
    Some(e)
}

fn monofasia_padrao(
    cbs: Option<&CbsIbsSaida>,
    ibs_uf: Option<&CbsIbsSaida>,
) -> Option<MonofasiaPadrao> {
    etapa_monofasia(cbs, ibs_uf, |g| g.monofasico.as_ref()).map(|e| MonofasiaPadrao {
        q_bc_mono: e.q_bc,
        ad_rem_ibs: e.ad_rem_ibs,
        ad_rem_cbs: e.ad_rem_cbs,
        v_ibs_mono: e.v_ibs,
        v_cbs_mono: e.v_cbs,
    })
}

fn monofasia_diferimento(
    cbs: Option<&CbsIbsSaida>,
    ibs_uf: Option<&CbsIbsSaida>,
) -> Option<MonofasiaDiferimento> {
    let ibs = diferido(ibs_uf)?;
    let mut d = MonofasiaDiferimento::default();
    if let Some(c) = diferido(cbs) {
        d.p_dif_cbs = c.percentual_diferimento.clone();
        d.v_cbs_mono_dif = c.valor_diferimento.clone();
    }
    d.p_dif_ibs = ibs.percentual_diferimento.clone();
    d.v_ibs_mono_dif = ibs.valor_diferimento.clone();
    Some(d)
}

fn diferido(s: Option<&CbsIbsSaida>) -> Option<&DiferimentoMonofasiaSaida> {
    s.and_then(|s| s.grupo_monofasia.as_ref())
        .and_then(|g| g.diferido.as_ref())
}

fn monofasia(cbs: Option<&CbsIbsSaida>, ibs_uf: Option<&CbsIbsSaida>) -> Option<Monofasia> {
    let padrao = monofasia_padrao(cbs, ibs_uf);
    let reten =
        etapa_monofasia(cbs, ibs_uf, |g| g.sujeito_retencao.as_ref()).map(MonofasiaRetencao);
    let ret = etapa_monofasia(cbs, ibs_uf, |g| g.retido.as_ref()).map(MonofasiaRetido);
    let dif = monofasia_diferimento(cbs, ibs_uf);
    if padrao.is_none() && reten.is_none() && ret.is_none() && dif.is_none() {
        return None;
    }
    let v = |o: Option<&Option<BigDecimal>>| ou_zero(o.and_then(Option::as_ref));
    let v_ibs =
        v(padrao.as_ref().map(|p| &p.v_ibs_mono)).add(&v(reten.as_ref().map(|r| &r.0.v_ibs)));
    let v_cbs =
        v(padrao.as_ref().map(|p| &p.v_cbs_mono)).add(&v(reten.as_ref().map(|r| &r.0.v_cbs)));
    let v_ibs = v_ibs.subtract(&v(dif.as_ref().map(|d| &d.v_ibs_mono_dif)));
    let v_cbs = v_cbs.subtract(&v(dif.as_ref().map(|d| &d.v_cbs_mono_dif)));
    Some(Monofasia {
        g_mono_padrao: padrao,
        g_mono_reten: reten,
        g_mono_ret: ret,
        g_mono_dif: dif,
        v_tot_ibs_mono_item: Some(v_ibs),
        v_tot_cbs_mono_item: Some(v_cbs),
    })
}
