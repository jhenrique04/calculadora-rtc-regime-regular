//! ROC (resultado da operação de consumo): DTOs de saída de `api.model.roc`

use crate::decimal::BigDecimal;
use crate::json::{ErroEscrita, Escrever, Escritor};
use crate::saida_json;

fn zero() -> Option<BigDecimal> {
    Some(BigDecimal::zero())
}

fn ou_zero(v: Option<&BigDecimal>) -> BigDecimal {
    v.cloned().unwrap_or_else(BigDecimal::zero)
}

#[derive(Debug, Clone, Default)]
pub struct Diferimento {
    pub p_dif: Option<BigDecimal>,
    pub v_dif: Option<BigDecimal>,
}

saida_json!(Diferimento {
    "pDif" => p_dif: aliquota,
    "vDif" => v_dif: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct DevolucaoTributos {
    pub p_dev_trib: Option<BigDecimal>,
    pub v_dev_trib: Option<BigDecimal>,
}

saida_json!(DevolucaoTributos {
    "pDevTrib" => p_dev_trib: aliquota,
    "vDevTrib" => v_dev_trib: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct ReducaoAliquota {
    pub p_red_aliq: Option<BigDecimal>,
    pub p_aliq_efet: Option<BigDecimal>,
}

saida_json!(ReducaoAliquota {
    "pRedAliq" => p_red_aliq: aliquota,
    "pAliqEfet" => p_aliq_efet: aliquota,
});

/// corpo comum de `IBSUFDomain`, `IBSMunDomain` e `CBSDomain`; só mudam os nomes
#[derive(Debug, Clone)]
pub struct GrupoTributo {
    pub aliquota: Option<BigDecimal>,
    pub g_dif: Option<Diferimento>,
    pub g_dev_trib: Option<DevolucaoTributos>,
    pub g_red: Option<ReducaoAliquota>,
    pub valor: Option<BigDecimal>,
    pub memoria_calculo: Option<String>,
}

impl Default for GrupoTributo {
    fn default() -> Self {
        Self {
            aliquota: zero(),
            g_dif: None,
            g_dev_trib: None,
            g_red: None,
            valor: zero(),
            memoria_calculo: None,
        }
    }
}

impl GrupoTributo {
    pub fn v_dif(&self) -> Option<BigDecimal> {
        match &self.g_dif {
            Some(d) => d.v_dif.clone(),
            None => zero(),
        }
    }

    pub fn v_dev_trib(&self) -> Option<BigDecimal> {
        match &self.g_dev_trib {
            Some(d) => d.v_dev_trib.clone(),
            None => zero(),
        }
    }

    fn escrever(&self, e: &mut Escritor, p: &str, v: &str) -> Result<(), ErroEscrita> {
        e.inicio_objeto();
        e.aliquota(p, &self.aliquota)?;
        e.objeto("gDif", &self.g_dif)?;
        e.objeto("gDevTrib", &self.g_dev_trib)?;
        e.objeto("gRed", &self.g_red)?;
        e.moeda(v, &self.valor)?;
        e.texto("memoriaCalculo", &self.memoria_calculo)?;
        e.fim_objeto();
        Ok(())
    }
}

macro_rules! grupo_tributo {
    ($tipo:ident, $p:literal, $v:literal) => {
        #[derive(Debug, Clone, Default)]
        pub struct $tipo(pub GrupoTributo);

        impl Escrever for $tipo {
            fn escrever(&self, e: &mut Escritor) -> Result<(), ErroEscrita> {
                self.0.escrever(e, $p, $v)
            }
        }
    };
}

grupo_tributo!(IbsUf, "pIBSUF", "vIBSUF");
grupo_tributo!(IbsMun, "pIBSMun", "vIBSMun");
grupo_tributo!(Cbs, "pCBS", "vCBS");

#[derive(Debug, Clone)]
pub struct TributacaoRegular {
    pub cst_reg: Option<String>,
    pub c_class_trib_reg: Option<String>,
    pub p_aliq_efet_reg_ibs_uf: Option<BigDecimal>,
    pub v_trib_reg_ibs_uf: Option<BigDecimal>,
    pub p_aliq_efet_reg_ibs_mun: Option<BigDecimal>,
    pub v_trib_reg_ibs_mun: Option<BigDecimal>,
    pub p_aliq_efet_reg_cbs: Option<BigDecimal>,
    pub v_trib_reg_cbs: Option<BigDecimal>,
}

impl Default for TributacaoRegular {
    fn default() -> Self {
        Self {
            cst_reg: None,
            c_class_trib_reg: None,
            p_aliq_efet_reg_ibs_uf: zero(),
            v_trib_reg_ibs_uf: zero(),
            p_aliq_efet_reg_ibs_mun: zero(),
            v_trib_reg_ibs_mun: zero(),
            p_aliq_efet_reg_cbs: zero(),
            v_trib_reg_cbs: zero(),
        }
    }
}

saida_json!(TributacaoRegular {
    "CSTReg" => cst_reg: cst,
    "cClassTribReg" => c_class_trib_reg: class_trib,
    "pAliqEfetRegIBSUF" => p_aliq_efet_reg_ibs_uf: aliquota,
    "vTribRegIBSUF" => v_trib_reg_ibs_uf: moeda,
    "pAliqEfetRegIBSMun" => p_aliq_efet_reg_ibs_mun: aliquota,
    "vTribRegIBSMun" => v_trib_reg_ibs_mun: moeda,
    "pAliqEfetRegCBS" => p_aliq_efet_reg_cbs: aliquota,
    "vTribRegCBS" => v_trib_reg_cbs: moeda,
});

#[derive(Debug, Clone)]
pub struct TributacaoCompraGov {
    pub p_aliq_ibs_uf: Option<BigDecimal>,
    pub v_trib_ibs_uf: Option<BigDecimal>,
    pub p_aliq_ibs_mun: Option<BigDecimal>,
    pub v_trib_ibs_mun: Option<BigDecimal>,
    pub p_aliq_cbs: Option<BigDecimal>,
    pub v_trib_cbs: Option<BigDecimal>,
}

impl Default for TributacaoCompraGov {
    fn default() -> Self {
        Self {
            p_aliq_ibs_uf: zero(),
            v_trib_ibs_uf: zero(),
            p_aliq_ibs_mun: zero(),
            v_trib_ibs_mun: zero(),
            p_aliq_cbs: zero(),
            v_trib_cbs: zero(),
        }
    }
}

impl TributacaoCompraGov {
    pub fn zerar_valores(&mut self) {
        *self = Self::default();
    }
}

saida_json!(TributacaoCompraGov {
    "pAliqIBSUF" => p_aliq_ibs_uf: aliquota,
    "vTribIBSUF" => v_trib_ibs_uf: moeda,
    "pAliqIBSMun" => p_aliq_ibs_mun: aliquota,
    "vTribIBSMun" => v_trib_ibs_mun: moeda,
    "pAliqCBS" => p_aliq_cbs: aliquota,
    "vTribCBS" => v_trib_cbs: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct GrupoIbsCbs {
    pub v_bc: Option<BigDecimal>,
    pub g_ibs_uf: Option<IbsUf>,
    pub g_ibs_mun: Option<IbsMun>,
    pub v_ibs: Option<BigDecimal>,
    pub g_cbs: Option<Cbs>,
    pub g_trib_regular: Option<TributacaoRegular>,
    pub g_trib_compra_gov: Option<TributacaoCompraGov>,
}

saida_json!(GrupoIbsCbs {
    "vBC" => v_bc: moeda,
    "gIBSUF" => g_ibs_uf: objeto,
    "gIBSMun" => g_ibs_mun: objeto,
    "vIBS" => v_ibs: moeda,
    "gCBS" => g_cbs: objeto,
    "gTribRegular" => g_trib_regular: objeto,
    "gTribCompraGov" => g_trib_compra_gov: objeto,
});

#[derive(Debug, Clone)]
pub struct MonofasiaPadrao {
    pub q_bc_mono: Option<BigDecimal>,
    pub ad_rem_ibs: Option<BigDecimal>,
    pub ad_rem_cbs: Option<BigDecimal>,
    pub v_ibs_mono: Option<BigDecimal>,
    pub v_cbs_mono: Option<BigDecimal>,
}

impl Default for MonofasiaPadrao {
    fn default() -> Self {
        Self {
            q_bc_mono: zero(),
            ad_rem_ibs: zero(),
            ad_rem_cbs: zero(),
            v_ibs_mono: zero(),
            v_cbs_mono: zero(),
        }
    }
}

saida_json!(MonofasiaPadrao {
    "qBCMono" => q_bc_mono: quantidade,
    "adRemIBS" => ad_rem_ibs: aliquota,
    "adRemCBS" => ad_rem_cbs: aliquota,
    "vIBSMono" => v_ibs_mono: moeda,
    "vCBSMono" => v_cbs_mono: moeda,
});

/// corpo de `MonofasiaRetencaoDomain` e `MonofasiaRetidoAnteriormenteDomain`
#[derive(Debug, Clone)]
pub struct EtapaMonofasia {
    pub q_bc: Option<BigDecimal>,
    pub ad_rem_ibs: Option<BigDecimal>,
    pub v_ibs: Option<BigDecimal>,
    pub ad_rem_cbs: Option<BigDecimal>,
    pub v_cbs: Option<BigDecimal>,
}

impl Default for EtapaMonofasia {
    fn default() -> Self {
        Self {
            q_bc: zero(),
            ad_rem_ibs: zero(),
            v_ibs: zero(),
            ad_rem_cbs: zero(),
            v_cbs: zero(),
        }
    }
}

impl EtapaMonofasia {
    fn escrever(&self, e: &mut Escritor, nomes: [&str; 5]) -> Result<(), ErroEscrita> {
        e.inicio_objeto();
        e.quantidade(nomes[0], &self.q_bc)?;
        e.aliquota(nomes[1], &self.ad_rem_ibs)?;
        e.moeda(nomes[2], &self.v_ibs)?;
        e.aliquota(nomes[3], &self.ad_rem_cbs)?;
        e.moeda(nomes[4], &self.v_cbs)?;
        e.fim_objeto();
        Ok(())
    }
}

macro_rules! etapa_monofasia {
    ($tipo:ident, $nomes:expr) => {
        #[derive(Debug, Clone, Default)]
        pub struct $tipo(pub EtapaMonofasia);

        impl Escrever for $tipo {
            fn escrever(&self, e: &mut Escritor) -> Result<(), ErroEscrita> {
                self.0.escrever(e, $nomes)
            }
        }
    };
}

etapa_monofasia!(
    MonofasiaRetencao,
    [
        "qBCMonoReten",
        "adRemIBSReten",
        "vIBSMonoReten",
        "adRemCBSReten",
        "vCBSMonoReten"
    ]
);
etapa_monofasia!(
    MonofasiaRetido,
    [
        "qBCMonoRet",
        "adRemIBSRet",
        "vIBSMonoRet",
        "adRemCBSRet",
        "vCBSMonoRet"
    ]
);

#[derive(Debug, Clone)]
pub struct MonofasiaDiferimento {
    pub p_dif_ibs: Option<BigDecimal>,
    pub v_ibs_mono_dif: Option<BigDecimal>,
    pub p_dif_cbs: Option<BigDecimal>,
    pub v_cbs_mono_dif: Option<BigDecimal>,
}

impl Default for MonofasiaDiferimento {
    fn default() -> Self {
        Self {
            p_dif_ibs: zero(),
            v_ibs_mono_dif: zero(),
            p_dif_cbs: zero(),
            v_cbs_mono_dif: zero(),
        }
    }
}

saida_json!(MonofasiaDiferimento {
    "pDifIBS" => p_dif_ibs: aliquota,
    "vIBSMonoDif" => v_ibs_mono_dif: moeda,
    "pDifCBS" => p_dif_cbs: aliquota,
    "vCBSMonoDif" => v_cbs_mono_dif: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct Monofasia {
    pub g_mono_padrao: Option<MonofasiaPadrao>,
    pub g_mono_reten: Option<MonofasiaRetencao>,
    pub g_mono_ret: Option<MonofasiaRetido>,
    pub g_mono_dif: Option<MonofasiaDiferimento>,
    pub v_tot_ibs_mono_item: Option<BigDecimal>,
    pub v_tot_cbs_mono_item: Option<BigDecimal>,
}

saida_json!(Monofasia {
    "gMonoPadrao" => g_mono_padrao: objeto,
    "gMonoReten" => g_mono_reten: objeto,
    "gMonoRet" => g_mono_ret: objeto,
    "gMonoDif" => g_mono_dif: objeto,
    "vTotIBSMonoItem" => v_tot_ibs_mono_item: moeda,
    "vTotCBSMonoItem" => v_tot_cbs_mono_item: moeda,
});

/// valores `getVxxx()` do `MonofasiaDomain` (zero quando o grupo falta)
struct ValoresMonofasia {
    v_ibs_mono: BigDecimal,
    v_cbs_mono: BigDecimal,
    v_ibs_mono_reten: BigDecimal,
    v_cbs_mono_reten: BigDecimal,
    v_ibs_mono_ret: BigDecimal,
    v_cbs_mono_ret: BigDecimal,
}

impl Monofasia {
    fn valores(&self) -> ValoresMonofasia {
        let padrao = self.g_mono_padrao.as_ref();
        let reten = self.g_mono_reten.as_ref().map(|g| &g.0);
        let ret = self.g_mono_ret.as_ref().map(|g| &g.0);
        ValoresMonofasia {
            v_ibs_mono: ou_zero(padrao.and_then(|g| g.v_ibs_mono.as_ref())),
            v_cbs_mono: ou_zero(padrao.and_then(|g| g.v_cbs_mono.as_ref())),
            v_ibs_mono_reten: ou_zero(reten.and_then(|g| g.v_ibs.as_ref())),
            v_cbs_mono_reten: ou_zero(reten.and_then(|g| g.v_cbs.as_ref())),
            v_ibs_mono_ret: ou_zero(ret.and_then(|g| g.v_ibs.as_ref())),
            v_cbs_mono_ret: ou_zero(ret.and_then(|g| g.v_cbs.as_ref())),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransferenciaCredito {
    pub v_ibs: Option<BigDecimal>,
    pub v_cbs: Option<BigDecimal>,
}

saida_json!(TransferenciaCredito {
    "vIBS" => v_ibs: moeda,
    "vCBS" => v_cbs: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct AjusteCompetencia {
    pub compet_apur: Option<String>,
    pub v_ibs: Option<BigDecimal>,
    pub v_cbs: Option<BigDecimal>,
}

saida_json!(AjusteCompetencia {
    "competApur" => compet_apur: texto,
    "vIBS" => v_ibs: moeda,
    "vCBS" => v_cbs: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct EstornoCredito {
    pub v_ibs_est_cred: Option<BigDecimal>,
    pub v_cbs_est_cred: Option<BigDecimal>,
}

saida_json!(EstornoCredito {
    "vIBSEstCred" => v_ibs_est_cred: moeda,
    "vCBSEstCred" => v_cbs_est_cred: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct IbsCbsCreditoPresumido {
    pub p_cred_pres: Option<BigDecimal>,
    pub v_cred_pres: Option<BigDecimal>,
    pub v_cred_pres_cond_sus: Option<BigDecimal>,
}

saida_json!(IbsCbsCreditoPresumido {
    "pCredPres" => p_cred_pres: aliquota,
    "vCredPres" => v_cred_pres: moeda,
    "vCredPresCondSus" => v_cred_pres_cond_sus: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct CreditoPresumidoOperacao {
    pub v_bc_cred_pres: Option<BigDecimal>,
    pub c_cred_pres: Option<i32>,
    pub g_ibs_cred_pres: Option<IbsCbsCreditoPresumido>,
    pub g_cbs_cred_pres: Option<IbsCbsCreditoPresumido>,
}

saida_json!(CreditoPresumidoOperacao {
    "vBCCredPres" => v_bc_cred_pres: moeda,
    "cCredPres" => c_cred_pres: inteiro,
    "gIBSCredPres" => g_ibs_cred_pres: objeto,
    "gCBSCredPres" => g_cbs_cred_pres: objeto,
});

#[derive(Debug, Clone)]
pub struct CreditoPresumidoIbsZfm {
    pub compet_apur: Option<String>,
    pub tp_cred_pres_ibs_zfm: Option<i32>,
    pub v_cred_pres_ibs_zfm: Option<BigDecimal>,
}

impl Default for CreditoPresumidoIbsZfm {
    fn default() -> Self {
        Self {
            compet_apur: None,
            tp_cred_pres_ibs_zfm: None,
            v_cred_pres_ibs_zfm: zero(),
        }
    }
}

saida_json!(CreditoPresumidoIbsZfm {
    "competApur" => compet_apur: texto,
    "tpCredPresIBSZFM" => tp_cred_pres_ibs_zfm: inteiro,
    "vCredPresIBSZFM" => v_cred_pres_ibs_zfm: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct IbsCbs {
    pub cst: Option<String>,
    pub c_class_trib: Option<String>,
    pub ind_doacao: Option<i32>,
    pub g_ibs_cbs: Option<GrupoIbsCbs>,
    pub g_ibs_cbs_mono: Option<Monofasia>,
    pub g_transf_cred: Option<TransferenciaCredito>,
    pub g_ajuste_compet: Option<AjusteCompetencia>,
    pub g_estorno_cred: Option<EstornoCredito>,
    pub g_cred_pres_oper: Option<CreditoPresumidoOperacao>,
    pub g_cred_pres_ibs_zfm: Option<CreditoPresumidoIbsZfm>,
}

saida_json!(IbsCbs {
    "CST" => cst: cst,
    "cClassTrib" => c_class_trib: class_trib,
    "indDoacao" => ind_doacao: inteiro,
    "gIBSCBS" => g_ibs_cbs: objeto,
    "gIBSCBSMono" => g_ibs_cbs_mono: objeto,
    "gTransfCred" => g_transf_cred: objeto,
    "gAjusteCompet" => g_ajuste_compet: objeto,
    "gEstornoCred" => g_estorno_cred: objeto,
    "gCredPresOper" => g_cred_pres_oper: objeto,
    "gCredPresIBSZFM" => g_cred_pres_ibs_zfm: objeto,
});

impl IbsCbs {
    fn cred_pres_ibs(&self) -> Option<&IbsCbsCreditoPresumido> {
        self.g_cred_pres_oper
            .as_ref()
            .and_then(|c| c.g_ibs_cred_pres.as_ref())
    }

    fn cred_pres_cbs(&self) -> Option<&IbsCbsCreditoPresumido> {
        self.g_cred_pres_oper
            .as_ref()
            .and_then(|c| c.g_cbs_cred_pres.as_ref())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImpostoSeletivo {
    pub cst_is: Option<String>,
    pub c_class_trib_is: Option<String>,
    pub v_bc_is: Option<BigDecimal>,
    pub p_is: Option<BigDecimal>,
    pub p_is_espec: Option<BigDecimal>,
    pub ad_rem_is: Option<BigDecimal>,
    pub u_trib: Option<String>,
    pub q_trib: Option<BigDecimal>,
    pub v_is: Option<BigDecimal>,
    pub memoria_calculo: Option<String>,
}

saida_json!(ImpostoSeletivo {
    "CSTIS" => cst_is: cst,
    "cClassTribIS" => c_class_trib_is: class_trib,
    "vBCIS" => v_bc_is: moeda,
    "pIS" => p_is: aliquota,
    "pISEspec" => p_is_espec: aliquota,
    "adRemIS" => ad_rem_is: aliquota,
    "uTrib" => u_trib: u_trib,
    "qTrib" => q_trib: quantidade,
    "vIS" => v_is: moeda,
    "memoriaCalculo" => memoria_calculo: texto,
});

impl ImpostoSeletivo {
    pub fn valor(&self) -> BigDecimal {
        ou_zero(self.v_is.as_ref())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Tributos {
    pub is: Option<ImpostoSeletivo>,
    pub ibs_cbs: Option<IbsCbs>,
}

saida_json!(Tributos {
    "IS" => is: objeto,
    "IBSCBS" => ibs_cbs: objeto,
});

impl Tributos {
    fn grupo(&self) -> Option<&GrupoIbsCbs> {
        self.ibs_cbs.as_ref().and_then(|t| t.g_ibs_cbs.as_ref())
    }

    pub fn tem_cbs(&self) -> bool {
        self.grupo().is_some_and(|g| g.g_cbs.is_some())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Objeto {
    pub n_obj: Option<i32>,
    pub trib_calc: Option<Tributos>,
    pub calculo_simulado: bool,
}

saida_json!(Objeto {
    "nObj" => n_obj: inteiro,
    "tribCalc" => trib_calc: objeto,
    "calculoSimulado" => calculo_simulado: booleano,
});

#[derive(Debug, Clone, Default)]
pub struct CompraGovSaida {
    pub tp_ente_gov: Option<u8>,
    pub p_redutor: Option<BigDecimal>,
    pub tp_oper_gov: Option<u8>,
}

saida_json!(CompraGovSaida {
    "tpEnteGov" => tp_ente_gov: inteiro,
    "pRedutor" => p_redutor: aliquota,
    "tpOperGov" => tp_oper_gov: inteiro,
});

#[derive(Debug, Clone, Default)]
pub struct OperacaoConsumo {
    pub g_compra_gov: Option<CompraGovSaida>,
}

saida_json!(OperacaoConsumo {
    "gCompraGov" => g_compra_gov: objeto,
});

#[derive(Debug, Clone, Default)]
pub struct Roc {
    pub oper: Option<OperacaoConsumo>,
    pub objetos: Option<Vec<Objeto>>,
    pub total: Option<ValoresTotais>,
}

saida_json!(Roc {
    "oper" => oper: objeto,
    "objetos" => objetos: lista,
    "total" => total: objeto,
});

#[derive(Debug, Clone, Default)]
pub struct ImpostoSeletivoTotal {
    pub v_is: Option<BigDecimal>,
}

saida_json!(ImpostoSeletivoTotal {
    "vIS" => v_is: moeda,
});

#[derive(Debug, Clone)]
pub struct TotalEnte {
    pub v_dif: Option<BigDecimal>,
    pub v_dev_trib: Option<BigDecimal>,
    pub valor: Option<BigDecimal>,
}

impl Default for TotalEnte {
    fn default() -> Self {
        Self {
            v_dif: zero(),
            v_dev_trib: zero(),
            valor: zero(),
        }
    }
}

impl TotalEnte {
    fn somar<'a>(grupos: impl Iterator<Item = &'a GrupoTributo>) -> Option<Self> {
        let mut total: Option<[BigDecimal; 3]> = None;
        for g in grupos {
            let parcelas = [
                ou_zero(g.v_dif().as_ref()),
                ou_zero(g.v_dev_trib().as_ref()),
                ou_zero(g.valor.as_ref()),
            ];
            total = Some(match total {
                None => parcelas,
                Some([a, b, c]) => [
                    a.add(&parcelas[0]),
                    b.add(&parcelas[1]),
                    c.add(&parcelas[2]),
                ],
            });
        }
        total.map(|[v_dif, v_dev_trib, valor]| Self {
            v_dif: Some(v_dif),
            v_dev_trib: Some(v_dev_trib),
            valor: Some(valor),
        })
    }

    fn escrever(&self, e: &mut Escritor, nome_valor: &str) -> Result<(), ErroEscrita> {
        e.inicio_objeto();
        e.moeda("vDif", &self.v_dif)?;
        e.moeda("vDevTrib", &self.v_dev_trib)?;
        e.moeda(nome_valor, &self.valor)?;
        e.fim_objeto();
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct IbsUfTotal(pub TotalEnte);

impl Escrever for IbsUfTotal {
    fn escrever(&self, e: &mut Escritor) -> Result<(), ErroEscrita> {
        self.0.escrever(e, "vIBSUF")
    }
}

#[derive(Debug, Clone, Default)]
pub struct IbsMunTotal(pub TotalEnte);

impl Escrever for IbsMunTotal {
    fn escrever(&self, e: &mut Escritor) -> Result<(), ErroEscrita> {
        self.0.escrever(e, "vIBSMun")
    }
}

#[derive(Debug, Clone, Default)]
pub struct IbsTotal {
    pub g_ibs_uf: Option<IbsUfTotal>,
    pub g_ibs_mun: Option<IbsMunTotal>,
    pub v_ibs: Option<BigDecimal>,
    pub v_cred_pres: Option<BigDecimal>,
    pub v_cred_pres_cond_sus: Option<BigDecimal>,
}

saida_json!(IbsTotal {
    "gIBSUF" => g_ibs_uf: objeto,
    "gIBSMun" => g_ibs_mun: objeto,
    "vIBS" => v_ibs: moeda,
    "vCredPres" => v_cred_pres: moeda,
    "vCredPresCondSus" => v_cred_pres_cond_sus: moeda,
});

#[derive(Debug, Clone)]
pub struct CbsTotal {
    pub v_dif: Option<BigDecimal>,
    pub v_dev_trib: Option<BigDecimal>,
    pub v_cbs: Option<BigDecimal>,
    pub v_cred_pres: Option<BigDecimal>,
    pub v_cred_pres_cond_sus: Option<BigDecimal>,
}

impl Default for CbsTotal {
    fn default() -> Self {
        Self {
            v_dif: zero(),
            v_dev_trib: zero(),
            v_cbs: zero(),
            v_cred_pres: zero(),
            v_cred_pres_cond_sus: zero(),
        }
    }
}

saida_json!(CbsTotal {
    "vDif" => v_dif: moeda,
    "vDevTrib" => v_dev_trib: moeda,
    "vCBS" => v_cbs: moeda,
    "vCredPres" => v_cred_pres: moeda,
    "vCredPresCondSus" => v_cred_pres_cond_sus: moeda,
});

#[derive(Debug, Clone)]
pub struct MonofasiaTotal {
    pub v_ibs_mono: Option<BigDecimal>,
    pub v_cbs_mono: Option<BigDecimal>,
    pub v_ibs_mono_reten: Option<BigDecimal>,
    pub v_cbs_mono_reten: Option<BigDecimal>,
    pub v_ibs_mono_ret: Option<BigDecimal>,
    pub v_cbs_mono_ret: Option<BigDecimal>,
}

impl Default for MonofasiaTotal {
    fn default() -> Self {
        Self {
            v_ibs_mono: zero(),
            v_cbs_mono: zero(),
            v_ibs_mono_reten: zero(),
            v_cbs_mono_reten: zero(),
            v_ibs_mono_ret: zero(),
            v_cbs_mono_ret: zero(),
        }
    }
}

saida_json!(MonofasiaTotal {
    "vIBSMono" => v_ibs_mono: moeda,
    "vCBSMono" => v_cbs_mono: moeda,
    "vIBSMonoReten" => v_ibs_mono_reten: moeda,
    "vCBSMonoReten" => v_cbs_mono_reten: moeda,
    "vIBSMonoRet" => v_ibs_mono_ret: moeda,
    "vCBSMonoRet" => v_cbs_mono_ret: moeda,
});

#[derive(Debug, Clone, Default)]
pub struct IbsCbsTotal {
    pub v_bc_ibs_cbs: Option<BigDecimal>,
    pub g_ibs: Option<IbsTotal>,
    pub g_cbs: Option<CbsTotal>,
    pub g_mono: Option<MonofasiaTotal>,
    pub g_estorno_cred: Option<EstornoCredito>,
}

saida_json!(IbsCbsTotal {
    "vBCIBSCBS" => v_bc_ibs_cbs: moeda,
    "gIBS" => g_ibs: objeto,
    "gCBS" => g_cbs: objeto,
    "gMono" => g_mono: objeto,
    "gEstornoCred" => g_estorno_cred: objeto,
});

#[derive(Debug, Clone, Default)]
pub struct TributosTotais {
    pub is_tot: Option<ImpostoSeletivoTotal>,
    pub ibs_cbs_tot: Option<IbsCbsTotal>,
}

saida_json!(TributosTotais {
    "ISTot" => is_tot: objeto,
    "IBSCBSTot" => ibs_cbs_tot: objeto,
});

#[derive(Debug, Clone, Default)]
pub struct ValoresTotais {
    pub trib_calc: Option<TributosTotais>,
}

saida_json!(ValoresTotais {
    "tribCalc" => trib_calc: objeto,
});

fn somar(valores: impl Iterator<Item = BigDecimal>) -> BigDecimal {
    valores.fold(BigDecimal::zero(), |acc, v| acc.add(&v))
}

impl ValoresTotais {
    /// `ValoresTotaisDomain.create`: soma só os tributos efetivamente presentes
    pub fn de(tributos: &[&Tributos]) -> Self {
        Self {
            trib_calc: Some(TributosTotais {
                is_tot: total_is(tributos),
                ibs_cbs_tot: total_ibs_cbs(tributos),
            }),
        }
    }
}

fn total_is(tributos: &[&Tributos]) -> Option<ImpostoSeletivoTotal> {
    let com_is: Vec<&ImpostoSeletivo> = tributos.iter().filter_map(|t| t.is.as_ref()).collect();
    if com_is.is_empty() {
        return None;
    }
    Some(ImpostoSeletivoTotal {
        v_is: Some(somar(com_is.iter().map(|i| i.valor()))),
    })
}

fn total_ibs_cbs(tributos: &[&Tributos]) -> Option<IbsCbsTotal> {
    if tributos.is_empty() {
        return None;
    }
    let grupos: Vec<&GrupoIbsCbs> = tributos.iter().filter_map(|t| t.grupo()).collect();
    let ibs_cbs: Vec<&IbsCbs> = tributos.iter().filter_map(|t| t.ibs_cbs.as_ref()).collect();
    Some(IbsCbsTotal {
        v_bc_ibs_cbs: Some(somar(grupos.iter().filter_map(|g| g.v_bc.clone()))),
        g_ibs: Some(total_ibs(&grupos, &ibs_cbs)),
        g_cbs: Some(total_cbs(&grupos, &ibs_cbs).unwrap_or_default()),
        g_mono: Some(total_monofasia(&ibs_cbs).unwrap_or_default()),
        g_estorno_cred: total_estorno(&ibs_cbs),
    })
}

fn total_ibs(grupos: &[&GrupoIbsCbs], ibs_cbs: &[&IbsCbs]) -> IbsTotal {
    let uf = TotalEnte::somar(
        grupos
            .iter()
            .filter_map(|g| g.g_ibs_uf.as_ref().map(|x| &x.0)),
    )
    .unwrap_or_default();
    let mun = TotalEnte::somar(
        grupos
            .iter()
            .filter_map(|g| g.g_ibs_mun.as_ref().map(|x| &x.0)),
    )
    .unwrap_or_default();
    let v_ibs = ou_zero(uf.valor.as_ref()).add(&ou_zero(mun.valor.as_ref()));
    let cred = ibs_cbs.iter().filter_map(|t| t.cred_pres_ibs());
    let v_cred_pres = somar(cred.clone().filter_map(|c| c.v_cred_pres.clone()));
    let v_cred_pres_cond_sus = somar(cred.filter_map(|c| c.v_cred_pres_cond_sus.clone()));
    IbsTotal {
        g_ibs_uf: Some(IbsUfTotal(uf)),
        g_ibs_mun: Some(IbsMunTotal(mun)),
        v_ibs: Some(v_ibs),
        v_cred_pres: Some(v_cred_pres),
        v_cred_pres_cond_sus: Some(v_cred_pres_cond_sus),
    }
}

fn total_cbs(grupos: &[&GrupoIbsCbs], ibs_cbs: &[&IbsCbs]) -> Option<CbsTotal> {
    let cbs: Vec<&GrupoTributo> = grupos
        .iter()
        .filter_map(|g| g.g_cbs.as_ref().map(|c| &c.0))
        .collect();
    let cred: Vec<&IbsCbsCreditoPresumido> =
        ibs_cbs.iter().filter_map(|t| t.cred_pres_cbs()).collect();
    if cbs.is_empty() && cred.is_empty() {
        return None;
    }
    Some(CbsTotal {
        v_dif: Some(somar(cbs.iter().map(|c| ou_zero(c.v_dif().as_ref())))),
        v_dev_trib: Some(somar(cbs.iter().map(|c| ou_zero(c.v_dev_trib().as_ref())))),
        v_cbs: Some(somar(cbs.iter().map(|c| ou_zero(c.valor.as_ref())))),
        v_cred_pres: Some(somar(cred.iter().map(|c| ou_zero(c.v_cred_pres.as_ref())))),
        v_cred_pres_cond_sus: Some(somar(
            cred.iter()
                .map(|c| ou_zero(c.v_cred_pres_cond_sus.as_ref())),
        )),
    })
}

fn total_monofasia(ibs_cbs: &[&IbsCbs]) -> Option<MonofasiaTotal> {
    let valores: Vec<ValoresMonofasia> = ibs_cbs
        .iter()
        .filter_map(|t| t.g_ibs_cbs_mono.as_ref())
        .map(Monofasia::valores)
        .collect();
    if valores.is_empty() {
        return None;
    }
    Some(MonofasiaTotal {
        v_ibs_mono: Some(somar(valores.iter().map(|v| v.v_ibs_mono.clone()))),
        v_cbs_mono: Some(somar(valores.iter().map(|v| v.v_cbs_mono.clone()))),
        v_ibs_mono_reten: Some(somar(valores.iter().map(|v| v.v_ibs_mono_reten.clone()))),
        v_cbs_mono_reten: Some(somar(valores.iter().map(|v| v.v_cbs_mono_reten.clone()))),
        v_ibs_mono_ret: Some(somar(valores.iter().map(|v| v.v_ibs_mono_ret.clone()))),
        v_cbs_mono_ret: Some(somar(valores.iter().map(|v| v.v_cbs_mono_ret.clone()))),
    })
}

fn total_estorno(ibs_cbs: &[&IbsCbs]) -> Option<EstornoCredito> {
    let estornos: Vec<&EstornoCredito> = ibs_cbs
        .iter()
        .filter_map(|t| t.g_estorno_cred.as_ref())
        .collect();
    if estornos.is_empty() {
        return None;
    }
    Some(EstornoCredito {
        v_ibs_est_cred: Some(somar(
            estornos.iter().map(|e| ou_zero(e.v_ibs_est_cred.as_ref())),
        )),
        v_cbs_est_cred: Some(somar(
            estornos.iter().map(|e| ou_zero(e.v_cbs_est_cred.as_ref())),
        )),
    })
}
