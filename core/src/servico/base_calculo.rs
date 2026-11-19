//! `BaseCalculoUnificadaService`: base de cálculo a partir dos componentes da NF-e/NFC-e

use crate::decimal::BigDecimal;
use crate::erro::{Classe, Falha, Resultado};
use crate::modelo::base_calculo::{BaseUnificadaEntrada, LayoutMercadorias, MercadoriasEntrada};
use crate::modelo::entrada::{ComponentesBc, ElementoBc};
use crate::saida_json;
use crate::util::arredondamento::subtrair;

const NFE: i32 = 55;
const NFCE: i32 = 65;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Natureza {
    Bem,
    Servico,
}

impl Natureza {
    fn nome(self) -> &'static str {
        match self {
            Self::Bem => "BEM",
            Self::Servico => "SERVICO",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operacao {
    Soma,
    Subtrai,
}

#[derive(Debug, Clone, Copy)]
struct Bases {
    rg: bool,
    is: bool,
    sn: bool,
}

const TODAS: Bases = Bases {
    rg: true,
    is: true,
    sn: true,
};
const RG_IS: Bases = Bases {
    rg: true,
    is: true,
    sn: false,
};
const RG_SN: Bases = Bases {
    rg: true,
    is: false,
    sn: true,
};
const ANO_INICIAL: i32 = 2026;
const ANO_FINAL: i32 = 2033;
const ANO_FIM_ICMS: i32 = 2032;
const ANO_INICIO_IS: i32 = 2027;
const ANO_FIM_PIS_COFINS: i32 = 2026;
const NFE_E_NFCE: &[i32] = &[NFE, NFCE];
const SO_NFE: &[i32] = &[NFE];

struct Regra {
    operacao: Operacao,
    bases: Bases,
    natureza: Option<Natureza>,
    ano_inicio: i32,
    ano_fim: i32,
    tipos_doc: &'static [i32],
}

/// tabela de `ElementoBaseCalculo`
fn regra(e: ElementoBc) -> Regra {
    use ElementoBc::*;
    let (operacao, bases, natureza, ano_inicio, ano_fim, tipos_doc) = match e {
        VProd | VFrete | VSeg | VOutro => (
            Operacao::Soma,
            TODAS,
            None,
            ANO_INICIAL,
            ANO_FINAL,
            NFE_E_NFCE,
        ),
        VIi => (
            Operacao::Soma,
            RG_IS,
            Some(Natureza::Bem),
            ANO_INICIAL,
            ANO_FINAL,
            SO_NFE,
        ),
        VIs => (
            Operacao::Soma,
            RG_SN,
            None,
            ANO_INICIO_IS,
            ANO_FINAL,
            NFE_E_NFCE,
        ),
        VDesc => (
            Operacao::Subtrai,
            TODAS,
            None,
            ANO_INICIAL,
            ANO_FINAL,
            NFE_E_NFCE,
        ),
        VPis | VCofins => (
            Operacao::Subtrai,
            RG_IS,
            None,
            ANO_INICIAL,
            ANO_FIM_PIS_COFINS,
            NFE_E_NFCE,
        ),
        VIcms | VFcp | VIcmsMono => (
            Operacao::Subtrai,
            RG_IS,
            Some(Natureza::Bem),
            ANO_INICIAL,
            ANO_FIM_ICMS,
            NFE_E_NFCE,
        ),
        VIcmsUfDest | VFcpUfDest => (
            Operacao::Subtrai,
            RG_IS,
            Some(Natureza::Bem),
            ANO_INICIAL,
            ANO_FIM_ICMS,
            SO_NFE,
        ),
        VIssqn => (
            Operacao::Subtrai,
            RG_IS,
            Some(Natureza::Servico),
            ANO_INICIAL,
            ANO_FIM_ICMS,
            NFE_E_NFCE,
        ),
    };
    Regra {
        operacao,
        bases,
        natureza,
        ano_inicio,
        ano_fim,
        tipos_doc,
    }
}

impl Regra {
    fn natureza_valida(&self, natureza: Option<Natureza>) -> bool {
        self.natureza.is_none() || self.natureza == natureza
    }

    fn ano_valido(&self, ano: i32) -> bool {
        ano >= self.ano_inicio && ano <= self.ano_fim
    }

    fn compoe(&self, ano: i32, natureza: Option<Natureza>, tp_doc: i32) -> bool {
        self.ano_valido(ano) && self.natureza_valida(natureza) && self.tipos_doc.contains(&tp_doc)
    }

    fn motivo(
        &self,
        tag: &str,
        ano: i32,
        natureza: Option<Natureza>,
        tp_doc: i32,
    ) -> Option<String> {
        if !self.ano_valido(ano) {
            return Some(format!(
                "Campo {tag} não compõe as bases para o ano {ano} (vigência {}–{}) e foi ignorado no cálculo",
                self.ano_inicio, self.ano_fim
            ));
        }
        if !self.natureza_valida(natureza) {
            return Some(format!(
                "Campo {tag} não compõe as bases para a natureza {} (aplicável apenas a {}) e foi ignorado no cálculo",
                natureza.map_or("null", Natureza::nome),
                self.natureza.map_or("null", Natureza::nome)
            ));
        }
        if !self.tipos_doc.contains(&tp_doc) {
            return Some(format!(
                "Campo {tag} não compõe as bases para o documento modelo {tp_doc} e foi ignorado no cálculo"
            ));
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct BasesCalculadas {
    pub base_rg: BigDecimal,
    pub base_is: BigDecimal,
    pub base_sn: BigDecimal,
    pub avisos: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BaseUnificadaSaida {
    pub base_rg: Option<BigDecimal>,
    pub base_is: Option<BigDecimal>,
    pub base_sn: Option<BigDecimal>,
    pub avisos: Option<Vec<String>>,
}

saida_json!(BaseUnificadaSaida {
    "baseRG" => base_rg: decimal,
    "baseIS" => base_is: decimal,
    "baseSN" => base_sn: decimal,
    "avisos" => avisos: lista,
});

impl From<BasesCalculadas> for BaseUnificadaSaida {
    fn from(b: BasesCalculadas) -> Self {
        Self {
            base_rg: Some(b.base_rg),
            base_is: Some(b.base_is),
            base_sn: Some(b.base_sn),
            avisos: Some(b.avisos),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BaseMercadoriasSaida {
    pub base_calculo: Option<BigDecimal>,
}

saida_json!(BaseMercadoriasSaida {
    "baseCalculo" => base_calculo: decimal,
});

const PACOTE_ENTRADA: &str = "br.gov.serpro.rtc.api.model.input.basecalculo";

fn getter(campo: &str) -> String {
    let mut c = campo.chars();
    let primeira = c
        .next()
        .map(|p| p.to_uppercase().collect::<String>())
        .unwrap_or_default();
    format!("get{primeira}{}", c.as_str())
}

/// cadeia `getA().add(getB()).add(...)` com as mensagens do NPE detalhado
fn somar_cadeia<L: LayoutMercadorias>(
    e: &MercadoriasEntrada<L>,
    campos: &[&str],
) -> Resultado<BigDecimal> {
    let mut total: Option<BigDecimal> = None;
    for campo in campos {
        let valor = e.valor(campo).as_ref();
        total = Some(match (total, valor) {
            (None, Some(v)) => v.clone(),
            (Some(t), Some(v)) => t.add(v),
            (None, None) => {
                return Err(Falha::sistema(format!(
                    "Cannot invoke \"java.math.BigDecimal.add(java.math.BigDecimal)\" because the return value of \"{PACOTE_ENTRADA}.{}.{}()\" is null",
                    L::CLASSE,
                    getter(campo)
                )));
            }
            (Some(_), None) => {
                return Err(Falha::sistema(
                    "Cannot read field \"intCompact\" because \"augend\" is null",
                ));
            }
        });
    }
    Ok(total.unwrap_or_else(BigDecimal::zero))
}

pub fn calcular_mercadorias<L: LayoutMercadorias>(
    e: &MercadoriasEntrada<L>,
) -> Resultado<BaseMercadoriasSaida> {
    let ano = e.ano_fato_gerador.ok_or_else(Falha::sem_mensagem)?;
    if ano < L::ANO_INICIO {
        return Err(Falha::new(Classe::CampoInvalido, L::MENSAGEM_ANO));
    }
    for grupo in L::EXTINTOS {
        let campos: Vec<(&str, &Option<BigDecimal>)> = grupo
            .campos
            .iter()
            .map(|(rotulo, campo)| (*rotulo, e.valor(campo)))
            .collect();
        validar_extintos(&campos, ano, grupo.ano)?;
    }
    let integra = somar_cadeia(e, L::INTEGRA)?;
    let nao_integra = somar_cadeia(e, L::NAO_INTEGRA)?;
    let base = integra.subtract(&nao_integra);
    if base.signum() < 0 {
        return Err(Falha::new(
            Classe::CampoInvalido,
            "Base de cálculo não pode ser negativa. A soma dos valores que não integram a base é maior que a soma dos valores que integram a base.",
        ));
    }
    Ok(BaseMercadoriasSaida {
        base_calculo: Some(base),
    })
}

pub fn calcular_unificada(e: &BaseUnificadaEntrada) -> Resultado<BaseUnificadaSaida> {
    let ano = e.ano_fato_gerador.ok_or_else(Falha::sem_mensagem)?;
    calcular(&e.componentes, ano, e.natureza, e.tipo_documento).map(Into::into)
}

#[derive(Default)]
struct Parcelas {
    soma: BigDecimal,
    subtrai: BigDecimal,
}

impl Parcelas {
    fn acumular(&mut self, operacao: Operacao, valor: &BigDecimal) {
        let alvo = match operacao {
            Operacao::Soma => &mut self.soma,
            Operacao::Subtrai => &mut self.subtrai,
        };
        *alvo = alvo.add(valor);
    }

    fn base(&self) -> BigDecimal {
        subtrair(&self.soma, &self.subtrai)
    }
}

/// `validaValoresForaLimiteVigenciaPermitido`: o `Map.of` não aceita nulos e a
/// lista sai em ordem alfabética
pub(crate) fn validar_extintos(
    campos: &[(&str, &Option<BigDecimal>)],
    ano: i32,
    ano_extincao: i32,
) -> Resultado<()> {
    if campos.iter().any(|(_, v)| v.is_none()) {
        return Err(Falha::sem_mensagem());
    }
    if ano < ano_extincao {
        return Ok(());
    }
    let mut informados: Vec<&str> = campos
        .iter()
        .filter(|(_, v)| v.as_ref().is_some_and(|d| d.signum() != 0))
        .map(|(n, _)| *n)
        .collect();
    informados.sort_unstable();
    if informados.is_empty() {
        return Ok(());
    }
    Err(Falha::new(
        Classe::CampoInvalido,
        format!(
            "Os seguintes campos não podem ser informados a partir de {ano_extincao}: {}",
            informados.join(", ")
        ),
    ))
}

pub fn calcular(
    componentes: &ComponentesBc,
    ano: i32,
    natureza: Option<Natureza>,
    tp_doc: Option<i32>,
) -> Resultado<BasesCalculadas> {
    let tp_doc = match tp_doc {
        Some(t) if t == NFE || t == NFCE => t,
        _ => {
            return Err(Falha::new(
                Classe::CampoInvalido,
                "Tipo de documento deve ser 55 (NF-e) ou 65 (NFC-e)",
            ));
        }
    };
    let (mut rg, mut is, mut sn) = (
        Parcelas::default(),
        Parcelas::default(),
        Parcelas::default(),
    );
    let mut avisos = Vec::new();
    for e in ElementoBc::TODOS {
        let r = regra(e);
        let Some(valor) = componentes.valor(e).filter(|v| !v.is_zero()) else {
            continue;
        };
        if !r.compoe(ano, natureza, tp_doc) {
            avisos.extend(r.motivo(e.tag(), ano, natureza, tp_doc));
            continue;
        }
        if r.bases.rg {
            rg.acumular(r.operacao, valor);
        }
        if r.bases.is {
            is.acumular(r.operacao, valor);
        }
        if r.bases.sn {
            sn.acumular(r.operacao, valor);
        }
    }
    Ok(BasesCalculadas {
        base_rg: rg.base(),
        base_is: is.base(),
        base_sn: sn.base(),
        avisos,
    })
}
