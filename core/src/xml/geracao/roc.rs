//! `ROCDomain` lido como o Jackson faz (tipos Java, mensagens e inicializadores
//! de campo), guardado em forma genérica: o ModelMapper casa por nome

use crate::decimal::BigDecimal;
use crate::json::{self, BooleanoPrimitivo, Leitor, LerJson, R, TextoBranco};
use crate::modelo::entrada::{TipoEnteGov, TipoOperacaoGov};

macro_rules! roc {
    ($classe:literal) => {
        concat!("br.gov.serpro.rtc.api.model.roc.", $classe)
    };
}

#[derive(Debug, Clone)]
pub enum Valor {
    Decimal(BigDecimal),
    Texto(String),
    Inteiro(i32),
    /// enum com `@JsonValue` numérico, guardado pelo código
    Codigo(u8),
    Booleano(bool),
    Objeto(Objeto),
    Lista(Vec<Option<Objeto>>),
}

/// propriedades presentes no JSON (ou com inicializador); `None` é `null` explícito
#[derive(Debug, Clone, Default)]
pub struct Objeto(Vec<(&'static str, Option<Valor>)>);

impl Objeto {
    pub fn valor(&self, nome: &str) -> Option<&Valor> {
        self.0
            .iter()
            .find(|(n, _)| *n == nome)
            .and_then(|(_, v)| v.as_ref())
    }

    pub fn objeto(&self, nome: &str) -> Option<&Objeto> {
        match self.valor(nome) {
            Some(Valor::Objeto(o)) => Some(o),
            _ => None,
        }
    }

    pub fn lista(&self, nome: &str) -> Option<&[Option<Objeto>]> {
        match self.valor(nome) {
            Some(Valor::Lista(l)) => Some(l),
            _ => None,
        }
    }

    pub fn inteiro(&self, nome: &str) -> Option<i32> {
        match self.valor(nome) {
            Some(Valor::Inteiro(i)) => Some(*i),
            _ => None,
        }
    }

    fn presente(&self, nome: &str) -> bool {
        self.0.iter().any(|(n, _)| *n == nome)
    }

    /// chave repetida no JSON: vale a última
    fn definir(&mut self, nome: &'static str, valor: Option<Valor>) {
        match self.0.iter_mut().find(|(n, _)| *n == nome) {
            Some(par) => par.1 = valor,
            None => self.0.push((nome, valor)),
        }
    }
}

#[derive(Clone, Copy)]
enum Tipo {
    Decimal,
    Texto,
    Inteiro,
    Booleano,
    EnteGov,
    OperGov,
    Objeto(&'static Classe),
    Lista(&'static Classe),
}

struct Propriedade {
    nome: &'static str,
    tipo: Tipo,
    /// `= ZERO` só vale em classe sem `@Builder` (construtor padrão + campos)
    zero: bool,
}

struct Classe {
    java: &'static str,
    propriedades: &'static [Propriedade],
}

const fn p(nome: &'static str, tipo: Tipo) -> Propriedade {
    Propriedade {
        nome,
        tipo,
        zero: false,
    }
}

const fn dec(nome: &'static str) -> Propriedade {
    p(nome, Tipo::Decimal)
}

const fn zero(nome: &'static str) -> Propriedade {
    Propriedade {
        nome,
        tipo: Tipo::Decimal,
        zero: true,
    }
}

const fn txt(nome: &'static str) -> Propriedade {
    p(nome, Tipo::Texto)
}

const fn int(nome: &'static str) -> Propriedade {
    p(nome, Tipo::Inteiro)
}

const fn obj(nome: &'static str, classe: &'static Classe) -> Propriedade {
    p(nome, Tipo::Objeto(classe))
}

const DIFERIMENTO: Classe = Classe {
    java: roc!("DiferimentoDomain"),
    propriedades: &[dec("pDif"), dec("vDif")],
};

const DEVOLUCAO: Classe = Classe {
    java: roc!("DevolucaoTributosDomain"),
    propriedades: &[dec("pDevTrib"), dec("vDevTrib")],
};

const REDUCAO: Classe = Classe {
    java: roc!("ReducaoAliquotaDomain"),
    propriedades: &[dec("pRedAliq"), dec("pAliqEfet")],
};

macro_rules! grupo_tributo {
    ($nome:ident, $classe:literal, $aliquota:literal, $valor:literal) => {
        const $nome: Classe = Classe {
            java: roc!($classe),
            propriedades: &[
                zero($aliquota),
                obj("gDif", &DIFERIMENTO),
                obj("gDevTrib", &DEVOLUCAO),
                obj("gRed", &REDUCAO),
                zero($valor),
                txt("memoriaCalculo"),
            ],
        };
    };
}

grupo_tributo!(IBS_UF, "IBSUFDomain", "pIBSUF", "vIBSUF");
grupo_tributo!(IBS_MUN, "IBSMunDomain", "pIBSMun", "vIBSMun");
grupo_tributo!(CBS, "CBSDomain", "pCBS", "vCBS");

const TRIBUTACAO_REGULAR: Classe = Classe {
    java: roc!("TributacaoRegularDomain"),
    propriedades: &[
        txt("CSTReg"),
        txt("cClassTribReg"),
        dec("pAliqEfetRegIBSUF"),
        dec("vTribRegIBSUF"),
        dec("pAliqEfetRegIBSMun"),
        dec("vTribRegIBSMun"),
        dec("pAliqEfetRegCBS"),
        dec("vTribRegCBS"),
    ],
};

const TRIBUTACAO_COMPRA_GOV: Classe = Classe {
    java: roc!("TributacaoCompraGovernamentalDomain"),
    propriedades: &[
        dec("pAliqIBSUF"),
        dec("vTribIBSUF"),
        dec("pAliqIBSMun"),
        dec("vTribIBSMun"),
        dec("pAliqCBS"),
        dec("vTribCBS"),
    ],
};

const GRUPO_IBS_CBS: Classe = Classe {
    java: roc!("GrupoIBSCBSDomain"),
    propriedades: &[
        dec("vBC"),
        obj("gIBSUF", &IBS_UF),
        obj("gIBSMun", &IBS_MUN),
        dec("vIBS"),
        obj("gCBS", &CBS),
        obj("gTribRegular", &TRIBUTACAO_REGULAR),
        obj("gTribCompraGov", &TRIBUTACAO_COMPRA_GOV),
    ],
};

const MONO_PADRAO: Classe = Classe {
    java: roc!("MonofasiaPadraoDomain"),
    propriedades: &[
        dec("qBCMono"),
        dec("adRemIBS"),
        dec("adRemCBS"),
        dec("vIBSMono"),
        dec("vCBSMono"),
    ],
};

const MONO_RETENCAO: Classe = Classe {
    java: roc!("MonofasiaRetencaoDomain"),
    propriedades: &[
        dec("qBCMonoReten"),
        dec("adRemIBSReten"),
        dec("vIBSMonoReten"),
        dec("adRemCBSReten"),
        dec("vCBSMonoReten"),
    ],
};

const MONO_RETIDO: Classe = Classe {
    java: roc!("MonofasiaRetidoAnteriormenteDomain"),
    propriedades: &[
        dec("qBCMonoRet"),
        dec("adRemIBSRet"),
        dec("vIBSMonoRet"),
        dec("adRemCBSRet"),
        dec("vCBSMonoRet"),
    ],
};

const MONO_DIFERIMENTO: Classe = Classe {
    java: roc!("MonofasiaDiferimentoDomain"),
    propriedades: &[
        dec("pDifIBS"),
        dec("vIBSMonoDif"),
        dec("pDifCBS"),
        dec("vCBSMonoDif"),
    ],
};

const MONOFASIA: Classe = Classe {
    java: roc!("MonofasiaDomain"),
    propriedades: &[
        obj("gMonoPadrao", &MONO_PADRAO),
        obj("gMonoReten", &MONO_RETENCAO),
        obj("gMonoRet", &MONO_RETIDO),
        obj("gMonoDif", &MONO_DIFERIMENTO),
        dec("vTotIBSMonoItem"),
        dec("vTotCBSMonoItem"),
    ],
};

const TRANSFERENCIA_CREDITO: Classe = Classe {
    java: roc!("TransferenciaCreditoDomain"),
    propriedades: &[dec("vIBS"), dec("vCBS")],
};

const AJUSTE_COMPETENCIA: Classe = Classe {
    java: roc!("AjusteCompetenciaDomain"),
    propriedades: &[txt("competApur"), dec("vIBS"), dec("vCBS")],
};

const ESTORNO_CREDITO: Classe = Classe {
    java: roc!("EstornoCreditoDomain"),
    propriedades: &[dec("vIBSEstCred"), dec("vCBSEstCred")],
};

const CREDITO_PRESUMIDO: Classe = Classe {
    java: roc!("IBSCBSCreditoPresumidoDomain"),
    propriedades: &[dec("pCredPres"), dec("vCredPres"), dec("vCredPresCondSus")],
};

const CREDITO_PRESUMIDO_OPERACAO: Classe = Classe {
    java: roc!("CreditoPresumidoOperacaoDomain"),
    propriedades: &[
        dec("vBCCredPres"),
        int("cCredPres"),
        obj("gIBSCredPres", &CREDITO_PRESUMIDO),
        obj("gCBSCredPres", &CREDITO_PRESUMIDO),
    ],
};

const CREDITO_PRESUMIDO_ZFM: Classe = Classe {
    java: roc!("CreditoPresumidoIBSZFMDomain"),
    propriedades: &[
        txt("competApur"),
        int("tpCredPresIBSZFM"),
        dec("vCredPresIBSZFM"),
    ],
};

const IBS_CBS: Classe = Classe {
    java: roc!("IBSCBSDomain"),
    propriedades: &[
        txt("CST"),
        txt("cClassTrib"),
        int("indDoacao"),
        obj("gIBSCBS", &GRUPO_IBS_CBS),
        obj("gIBSCBSMono", &MONOFASIA),
        obj("gTransfCred", &TRANSFERENCIA_CREDITO),
        obj("gAjusteCompet", &AJUSTE_COMPETENCIA),
        obj("gEstornoCred", &ESTORNO_CREDITO),
        obj("gCredPresOper", &CREDITO_PRESUMIDO_OPERACAO),
        obj("gCredPresIBSZFM", &CREDITO_PRESUMIDO_ZFM),
    ],
};

const IMPOSTO_SELETIVO: Classe = Classe {
    java: roc!("ImpostoSeletivoDomain"),
    propriedades: &[
        txt("CSTIS"),
        txt("cClassTribIS"),
        dec("vBCIS"),
        dec("pIS"),
        dec("pISEspec"),
        dec("adRemIS"),
        txt("uTrib"),
        dec("qTrib"),
        dec("vIS"),
        txt("memoriaCalculo"),
    ],
};

const TRIBUTOS: Classe = Classe {
    java: roc!("TributosDomain"),
    propriedades: &[obj("IS", &IMPOSTO_SELETIVO), obj("IBSCBS", &IBS_CBS)],
};

const OBJETO: Classe = Classe {
    java: roc!("ObjetoDomain"),
    propriedades: &[
        int("nObj"),
        obj("tribCalc", &TRIBUTOS),
        p("calculoSimulado", Tipo::Booleano),
    ],
};

const COMPRA_GOV: Classe = Classe {
    java: roc!("CompraGovernamentalDomain"),
    propriedades: &[
        p("tpEnteGov", Tipo::EnteGov),
        zero("pRedutor"),
        p("tpOperGov", Tipo::OperGov),
    ],
};

const OPERACAO: Classe = Classe {
    java: roc!("OperacaoConsumoDomain"),
    propriedades: &[obj("gCompraGov", &COMPRA_GOV)],
};

const IS_TOTAL: Classe = Classe {
    java: roc!("ImpostoSeletivoTotalDomain"),
    propriedades: &[zero("vIS")],
};

const IBS_UF_TOTAL: Classe = Classe {
    java: roc!("IBSUFTotalDomain"),
    propriedades: &[zero("vDif"), zero("vDevTrib"), zero("vIBSUF")],
};

const IBS_MUN_TOTAL: Classe = Classe {
    java: roc!("IBSMunTotalDomain"),
    propriedades: &[zero("vDif"), zero("vDevTrib"), zero("vIBSMun")],
};

const IBS_TOTAL: Classe = Classe {
    java: roc!("IBSTotalDomain"),
    propriedades: &[
        obj("gIBSUF", &IBS_UF_TOTAL),
        obj("gIBSMun", &IBS_MUN_TOTAL),
        dec("vIBS"),
        dec("vCredPres"),
        dec("vCredPresCondSus"),
    ],
};

const CBS_TOTAL: Classe = Classe {
    java: roc!("CBSTotalDomain"),
    propriedades: &[
        zero("vDif"),
        zero("vDevTrib"),
        zero("vCBS"),
        zero("vCredPres"),
        zero("vCredPresCondSus"),
    ],
};

const MONOFASIA_TOTAL: Classe = Classe {
    java: roc!("MonofasiaTotalDomain"),
    propriedades: &[
        zero("vIBSMono"),
        zero("vCBSMono"),
        zero("vIBSMonoReten"),
        zero("vCBSMonoReten"),
        zero("vIBSMonoRet"),
        zero("vCBSMonoRet"),
    ],
};

const IBS_CBS_TOTAL: Classe = Classe {
    java: roc!("IBSCBSTotalDomain"),
    propriedades: &[
        dec("vBCIBSCBS"),
        obj("gIBS", &IBS_TOTAL),
        obj("gCBS", &CBS_TOTAL),
        obj("gMono", &MONOFASIA_TOTAL),
        obj("gEstornoCred", &ESTORNO_CREDITO),
    ],
};

const TRIBUTOS_TOTAIS: Classe = Classe {
    java: roc!("TributosTotaisDomain"),
    propriedades: &[obj("ISTot", &IS_TOTAL), obj("IBSCBSTot", &IBS_CBS_TOTAL)],
};

const VALORES_TOTAIS: Classe = Classe {
    java: roc!("ValoresTotaisDomain"),
    propriedades: &[obj("tribCalc", &TRIBUTOS_TOTAIS)],
};

const ROC: Classe = Classe {
    java: roc!("ROCDomain"),
    propriedades: &[
        obj("oper", &OPERACAO),
        p("objetos", Tipo::Lista(&OBJETO)),
        obj("total", &VALORES_TOTAIS),
    ],
};

fn ler_valor(tipo: Tipo, l: &mut Leitor<'_>) -> R<Option<Valor>> {
    Ok(match tipo {
        Tipo::Decimal => BigDecimal::ler(l)?.map(Valor::Decimal),
        Tipo::Texto => String::ler(l)?.map(Valor::Texto),
        Tipo::Inteiro => i32::ler(l)?.map(Valor::Inteiro),
        Tipo::Booleano => BooleanoPrimitivo::ler(l)?.map(|b| Valor::Booleano(b.0)),
        Tipo::EnteGov => TipoEnteGov::ler(l)?.map(|e| Valor::Codigo(e.codigo())),
        Tipo::OperGov => TipoOperacaoGov::ler(l)?.map(|o| Valor::Codigo(o.0)),
        Tipo::Objeto(classe) => ler_objeto(classe, l)?.map(Valor::Objeto),
        Tipo::Lista(classe) => {
            let tipo_lista = format!("java.util.ArrayList<{}>", classe.java);
            json::ler_lista(l, &tipo_lista, |l| ler_objeto(classe, l))?.map(Valor::Lista)
        }
    })
}

fn ler_objeto(classe: &'static Classe, l: &mut Leitor<'_>) -> R<Option<Objeto>> {
    let lido = json::ler_objeto_com(
        l,
        classe.java,
        TextoBranco::ComoVazio,
        |o: &mut Objeto, nome, l| {
            let Some(prop) = classe.propriedades.iter().find(|p| p.nome == nome) else {
                return Ok(false);
            };
            o.definir(prop.nome, ler_valor(prop.tipo, l)?);
            Ok(true)
        },
    )?;
    Ok(lido.map(|mut o| {
        for prop in classe.propriedades.iter().filter(|p| p.zero) {
            if !o.presente(prop.nome) {
                o.definir(prop.nome, Some(Valor::Decimal(BigDecimal::zero())));
            }
        }
        o
    }))
}

/// corpo do `POST /calculadora/xml/generate`
#[derive(Debug, Clone, Default)]
pub struct Roc(pub Objeto);

impl LerJson for Roc {
    fn tipo_java() -> String {
        ROC.java.into()
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        Ok(ler_objeto(&ROC, l)?.map(Roc))
    }
}
