//! `XmlService.gerarXml`: ROC → classes JAXB de cada documento (via ModelMapper,
//! que casa propriedades pelo nome) → XML formatado

mod jaxb;
mod roc;

pub use roc::Roc;

use super::TipoDocumento;
use crate::erro::{Falha, Resultado};
use crate::util::arredondamento::formatar_aliquota;
use jaxb::Elemento;
use roc::{Objeto, Valor};

const NS_NFE: &str = "http://www.portalfiscal.inf.br/nfe";
const NS_CTE: &str = "http://www.portalfiscal.inf.br/cte";
const NS_BPE: &str = "http://www.portalfiscal.inf.br/bpe";
const NS_NF3E: &str = "http://www.portalfiscal.inf.br/nf3e";

const SEM_OBJETOS: &str = "ROC deve conter ao menos um objeto";
const SEM_ITENS: &str = "ROC deve conter ao menos um item";
const UM_ITEM: &str = "ROC deve conter um item";
const SEM_TOTAL: &str = "Total do ROC não informado";
// mensagens dos NullPointerException e do `ModelMapper.map(null, ...)`
const ORIGEM_NULA: &str = "source cannot be null";
const OBJETO_NULO: &str = "Cannot invoke \"br.gov.serpro.rtc.api.model.roc.ObjetoDomain.getNObj()\" because \"r\" is null";
const NOBJ_NULO: &str = "Cannot invoke \"java.lang.Integer.toString()\" because the return value of \"br.gov.serpro.rtc.api.model.roc.ObjetoDomain.getNObj()\" is null";
const PRIMEIRO_NULO: &str = "Cannot invoke \"br.gov.serpro.rtc.api.model.roc.ObjetoDomain.getTribCalc()\" because the return value of \"java.util.List.getFirst()\" is null";
const TRIB_CALC_NULO: &str = "Cannot invoke \"br.gov.serpro.rtc.api.model.roc.TributosDomain.getIBSCBS()\" because the return value of \"br.gov.serpro.rtc.api.model.roc.ObjetoDomain.getTribCalc()\" is null";

/// propriedade de uma classe JAXB, com o nome da propriedade de origem
enum Campo {
    Texto(&'static str),
    /// getter do ROC que troca `null` por zero (`MonofasiaDomain`)
    ZeroSeNulo(&'static str),
    /// conversor registrado para `pRedutor` (`ArredondamentoUtils.formatarAliquota`)
    Aliquota(&'static str),
    Grupo(&'static str, &'static [Campo]),
}

use Campo::{Aliquota, Grupo, Texto, ZeroSeNulo};

const DIF: &[Campo] = &[Texto("pDif"), Texto("vDif")];
const DEV_TRIB: &[Campo] = &[Texto("vDevTrib")];
const RED: &[Campo] = &[Texto("pRedAliq"), Texto("pAliqEfet")];

const IBS_UF: &[Campo] = &[
    Texto("pIBSUF"),
    Grupo("gDif", DIF),
    Grupo("gDevTrib", DEV_TRIB),
    Grupo("gRed", RED),
    Texto("vIBSUF"),
];

const IBS_MUN: &[Campo] = &[
    Texto("pIBSMun"),
    Grupo("gDif", DIF),
    Grupo("gDevTrib", DEV_TRIB),
    Grupo("gRed", RED),
    Texto("vIBSMun"),
];

const CBS: &[Campo] = &[
    Texto("pCBS"),
    Grupo("gDif", DIF),
    Grupo("gDevTrib", DEV_TRIB),
    Grupo("gRed", RED),
    Texto("vCBS"),
];

const TRIB_REGULAR: &[Campo] = &[
    Texto("CSTReg"),
    Texto("cClassTribReg"),
    Texto("pAliqEfetRegIBSUF"),
    Texto("vTribRegIBSUF"),
    Texto("pAliqEfetRegIBSMun"),
    Texto("vTribRegIBSMun"),
    Texto("pAliqEfetRegCBS"),
    Texto("vTribRegCBS"),
];

const TRIB_COMPRA_GOV: &[Campo] = &[
    Texto("pAliqIBSUF"),
    Texto("vTribIBSUF"),
    Texto("pAliqIBSMun"),
    Texto("vTribIBSMun"),
    Texto("pAliqCBS"),
    Texto("vTribCBS"),
];

const CIBS: &[Campo] = &[
    Texto("vBC"),
    Grupo("gIBSUF", IBS_UF),
    Grupo("gIBSMun", IBS_MUN),
    Texto("vIBS"),
    Grupo("gCBS", CBS),
    Grupo("gTribRegular", TRIB_REGULAR),
    Grupo("gTribCompraGov", TRIB_COMPRA_GOV),
];

const MONOFASIA: &[Campo] = &[
    Grupo(
        "gMonoPadrao",
        &[
            Texto("qBCMono"),
            Texto("adRemIBS"),
            Texto("adRemCBS"),
            Texto("vIBSMono"),
            Texto("vCBSMono"),
        ],
    ),
    Grupo(
        "gMonoReten",
        &[
            Texto("qBCMonoReten"),
            Texto("adRemIBSReten"),
            Texto("vIBSMonoReten"),
            Texto("adRemCBSReten"),
            Texto("vCBSMonoReten"),
        ],
    ),
    Grupo(
        "gMonoRet",
        &[
            Texto("qBCMonoRet"),
            Texto("adRemIBSRet"),
            Texto("vIBSMonoRet"),
            Texto("adRemCBSRet"),
            Texto("vCBSMonoRet"),
        ],
    ),
    Grupo(
        "gMonoDif",
        &[
            Texto("pDifIBS"),
            Texto("vIBSMonoDif"),
            Texto("pDifCBS"),
            Texto("vCBSMonoDif"),
        ],
    ),
    ZeroSeNulo("vTotIBSMonoItem"),
    ZeroSeNulo("vTotCBSMonoItem"),
];

const ESTORNO_CRED: &[Campo] = &[Texto("vIBSEstCred"), Texto("vCBSEstCred")];
const CRED_PRES: &[Campo] = &[
    Texto("pCredPres"),
    Texto("vCredPres"),
    Texto("vCredPresCondSus"),
];

// competApur (String → XMLGregorianCalendar) não tem conversor no ModelMapper e fica fora
const TRIB_NFE: &[Campo] = &[
    Texto("CST"),
    Texto("cClassTrib"),
    Texto("indDoacao"),
    Grupo("gIBSCBS", CIBS),
    Grupo("gIBSCBSMono", MONOFASIA),
    Grupo("gTransfCred", &[Texto("vIBS"), Texto("vCBS")]),
    Grupo("gAjusteCompet", &[Texto("vIBS"), Texto("vCBS")]),
    Grupo("gEstornoCred", ESTORNO_CRED),
    Grupo(
        "gCredPresOper",
        &[
            Texto("vBCCredPres"),
            Texto("cCredPres"),
            Grupo("gIBSCredPres", CRED_PRES),
            Grupo("gCBSCredPres", CRED_PRES),
        ],
    ),
    Grupo(
        "gCredPresIBSZFM",
        &[Texto("tpCredPresIBSZFM"), Texto("vCredPresIBSZFM")],
    ),
];

const TRIB_NFCE: &[Campo] = &[
    Texto("CST"),
    Texto("cClassTrib"),
    Texto("indDoacao"),
    Grupo("gIBSCBS", CIBS),
    Grupo("gIBSCBSMono", MONOFASIA),
];

const TRIB_CTE_BPE: &[Campo] = &[
    Texto("CST"),
    Texto("cClassTrib"),
    Texto("indDoacao"),
    Grupo("gIBSCBS", CIBS),
    Grupo("gEstornoCred", ESTORNO_CRED),
];

// indDoacao é o enum TIndDoacao: Integer → enum não converte e fica nulo
const TRIB_NF3E: &[Campo] = &[
    Texto("CST"),
    Texto("cClassTrib"),
    Grupo("gIBSCBS", CIBS),
    Grupo("gEstornoCred", ESTORNO_CRED),
];

const IS: &[Campo] = &[
    Texto("CSTIS"),
    Texto("cClassTribIS"),
    Texto("vBCIS"),
    Texto("pIS"),
    Texto("pISEspec"),
    Texto("uTrib"),
    Texto("qTrib"),
    Texto("vIS"),
];

const IMPOSTO_NFE: &[Campo] = &[Grupo("IS", IS), Grupo("IBSCBS", TRIB_NFE)];
const IMPOSTO_NFCE: &[Campo] = &[Grupo("IS", IS), Grupo("IBSCBS", TRIB_NFCE)];
const IMPOSTO_NF3E: &[Campo] = &[Grupo("IBSCBS", TRIB_NF3E)];
const IMP_BPE_TM: &[Campo] = &[Grupo("IBSCBS", TRIB_CTE_BPE)];

const COMPRA_GOV: &[Campo] = &[Texto("tpEnteGov"), Aliquota("pRedutor"), Texto("tpOperGov")];

const TOTAL_UF: &[Campo] = &[Texto("vDif"), Texto("vDevTrib"), Texto("vIBSUF")];
const TOTAL_MUN: &[Campo] = &[Texto("vDif"), Texto("vDevTrib"), Texto("vIBSMun")];

const TOTAL_NFE: &[Campo] = &[
    Grupo("ISTot", &[Texto("vIS")]),
    Grupo(
        "IBSCBSTot",
        &[
            Texto("vBCIBSCBS"),
            Grupo(
                "gIBS",
                &[
                    Grupo("gIBSUF", TOTAL_UF),
                    Grupo("gIBSMun", TOTAL_MUN),
                    Texto("vIBS"),
                    Texto("vCredPres"),
                    Texto("vCredPresCondSus"),
                ],
            ),
            Grupo(
                "gCBS",
                &[
                    Texto("vDif"),
                    Texto("vDevTrib"),
                    Texto("vCBS"),
                    Texto("vCredPres"),
                    Texto("vCredPresCondSus"),
                ],
            ),
            Grupo(
                "gMono",
                &[
                    Texto("vIBSMono"),
                    Texto("vCBSMono"),
                    Texto("vIBSMonoReten"),
                    Texto("vCBSMonoReten"),
                    Texto("vIBSMonoRet"),
                    Texto("vCBSMonoRet"),
                ],
            ),
            Grupo("gEstornoCred", ESTORNO_CRED),
        ],
    ),
];

/// `TIBSCBSTot` do BP-e TM e da NF3e
const TOTAL_IBS_CBS: &[Campo] = &[Grupo(
    "IBSCBSTot",
    &[
        Texto("vBCIBSCBS"),
        Grupo(
            "gIBS",
            &[
                Grupo("gIBSUF", TOTAL_UF),
                Grupo("gIBSMun", TOTAL_MUN),
                Texto("vIBS"),
            ],
        ),
        Grupo("gCBS", &[Texto("vDif"), Texto("vDevTrib"), Texto("vCBS")]),
        Grupo("gEstornoCred", ESTORNO_CRED),
    ],
)];

fn escalar(v: &Valor) -> Option<String> {
    match v {
        Valor::Decimal(d) => Some(d.to_string()),
        Valor::Texto(s) => Some(s.clone()),
        Valor::Inteiro(i) => Some(i.to_string()),
        Valor::Codigo(c) => Some(c.to_string()),
        Valor::Booleano(b) => Some(b.to_string()),
        Valor::Objeto(_) | Valor::Lista(_) => None,
    }
}

fn mapear(nome: &'static str, origem: &Objeto, campos: &[Campo]) -> Elemento {
    let filhos = campos
        .iter()
        .filter_map(|c| propriedade(origem, c))
        .collect();
    Elemento::grupo(nome, filhos)
}

fn propriedade(origem: &Objeto, campo: &Campo) -> Option<Elemento> {
    match *campo {
        Texto(nome) => origem
            .valor(nome)
            .and_then(escalar)
            .map(|v| Elemento::texto(nome, v)),
        ZeroSeNulo(nome) => {
            let v = origem.valor(nome).and_then(escalar);
            Some(Elemento::texto(nome, v.unwrap_or_else(|| "0".into())))
        }
        Aliquota(nome) => match origem.valor(nome) {
            Some(Valor::Decimal(d)) => Some(Elemento::texto(nome, formatar_aliquota(d))),
            _ => None,
        },
        Grupo(nome, campos) => origem.objeto(nome).map(|o| mapear(nome, o, campos)),
    }
}

fn falha(mensagem: &str) -> Falha {
    Falha::sistema(mensagem)
}

fn ao_menos_um<'a>(roc: &'a Objeto, mensagem: &str) -> Resultado<&'a [Option<Objeto>]> {
    roc.lista("objetos")
        .filter(|l| !l.is_empty())
        .ok_or_else(|| falha(mensagem))
}

fn um_unico(roc: &Objeto) -> Resultado<Option<&Objeto>> {
    match roc.lista("objetos") {
        Some([unico]) => Ok(unico.as_ref()),
        _ => Err(falha(UM_ITEM)),
    }
}

fn total(roc: &Objeto) -> Resultado<&Objeto> {
    roc.objeto("total")
        .and_then(|t| t.objeto("tribCalc"))
        .ok_or_else(|| falha(SEM_TOTAL))
}

/// `operToIde`: só existe com `oper.gCompraGov`
fn ide(roc: &Objeto) -> Option<Elemento> {
    let compra = roc.objeto("oper")?.objeto("gCompraGov")?;
    Some(Elemento::grupo(
        "ide",
        vec![mapear("gCompraGov", compra, COMPRA_GOV)],
    ))
}

/// `objetoToDet`: número do item primeiro, depois os tributos
fn det(
    item: Option<&Objeto>,
    atributo: &'static str,
    conteudo: fn(&Objeto) -> Elemento,
) -> Resultado<Elemento> {
    let objeto = item.ok_or_else(|| falha(OBJETO_NULO))?;
    let numero = objeto.inteiro("nObj").ok_or_else(|| falha(NOBJ_NULO))?;
    let tributos = objeto
        .objeto("tribCalc")
        .ok_or_else(|| falha(ORIGEM_NULA))?;
    Ok(Elemento::grupo("det", vec![conteudo(tributos)]).com_atributo(atributo, numero.to_string()))
}

fn dets(
    objetos: &[Option<Objeto>],
    atributo: &'static str,
    conteudo: fn(&Objeto) -> Elemento,
) -> Resultado<Vec<Elemento>> {
    objetos
        .iter()
        .map(|o| det(o.as_ref(), atributo, conteudo))
        .collect()
}

fn nfe(roc: &Objeto) -> Resultado<Elemento> {
    let objetos = ao_menos_um(roc, SEM_OBJETOS)?;
    let total = total(roc)?;
    let mut filhos: Vec<Elemento> = ide(roc).into_iter().collect();
    filhos.extend(dets(objetos, "nItem", |t| {
        mapear("imposto", t, IMPOSTO_NFE)
    })?);
    filhos.push(mapear("total", total, TOTAL_NFE));
    Ok(Elemento::grupo("infNFe", filhos))
}

fn nfce(roc: &Objeto) -> Resultado<Elemento> {
    let objetos = ao_menos_um(roc, SEM_OBJETOS)?;
    let total = total(roc)?;
    let mut filhos = dets(objetos, "nItem", |t| mapear("imposto", t, IMPOSTO_NFCE))?;
    filhos.push(mapear("total", total, TOTAL_NFE));
    Ok(Elemento::grupo("infNFe", filhos))
}

/// CT-e, CT-e simplificado e BP-e: só o IBSCBS do único item
fn so_ibs_cbs(roc: &Objeto, raiz: &'static str) -> Resultado<Elemento> {
    let item = um_unico(roc)?.ok_or_else(|| falha(PRIMEIRO_NULO))?;
    let tributos = item
        .objeto("tribCalc")
        .ok_or_else(|| falha(TRIB_CALC_NULO))?;
    let ibs_cbs = tributos
        .objeto("IBSCBS")
        .ok_or_else(|| falha(ORIGEM_NULA))?;
    let imp = Elemento::grupo("imp", vec![mapear("IBSCBS", ibs_cbs, TRIB_CTE_BPE)]);
    let mut filhos: Vec<Elemento> = ide(roc).into_iter().collect();
    filhos.push(imp);
    Ok(Elemento::grupo(raiz, filhos))
}

fn bpe_tm(roc: &Objeto) -> Resultado<Elemento> {
    let item = um_unico(roc)?;
    let total = total(roc)?;
    let mut filhos: Vec<Elemento> = ide(roc).into_iter().collect();
    let det = det(item, "nViagem", |t| mapear("imp", t, IMP_BPE_TM))?;
    filhos.push(Elemento::grupo("detBPeTM", vec![det]));
    filhos.push(mapear("total", total, TOTAL_IBS_CBS));
    Ok(Elemento::grupo("infBPe", filhos))
}

fn nf3e(roc: &Objeto) -> Resultado<Elemento> {
    let objetos = ao_menos_um(roc, SEM_ITENS)?;
    let total = total(roc)?;
    let dets = dets(objetos, "nItem", |t| {
        Elemento::grupo("detItem", vec![mapear("imposto", t, IMPOSTO_NF3E)])
    })?;
    let mut filhos: Vec<Elemento> = ide(roc).into_iter().collect();
    filhos.push(Elemento::grupo("NFdet", dets));
    filhos.push(mapear("total", total, TOTAL_IBS_CBS));
    Ok(Elemento::grupo("infNF3e", filhos))
}

pub fn gerar(roc: &Roc, tipo: TipoDocumento) -> Resultado<String> {
    let roc = &roc.0;
    let (raiz, namespace) = match tipo {
        TipoDocumento::Nfe => (nfe(roc)?, NS_NFE),
        TipoDocumento::Nfce => (nfce(roc)?, NS_NFE),
        TipoDocumento::Cte => (so_ibs_cbs(roc, "infCte")?, NS_CTE),
        TipoDocumento::CteSimplificado => (so_ibs_cbs(roc, "infCte")?, NS_CTE),
        TipoDocumento::Bpe => (so_ibs_cbs(roc, "infBPe")?, NS_BPE),
        TipoDocumento::BpeTm => (bpe_tm(roc)?, NS_BPE),
        TipoDocumento::Nf3e => (nf3e(roc)?, NS_NF3E),
        TipoDocumento::Nfse => {
            return Err(falha("Tipo de documento não suportado: NFSE"));
        }
    };
    Ok(jaxb::documento(&raiz, namespace))
}

#[cfg(test)]
mod tests;
