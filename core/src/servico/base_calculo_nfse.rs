//! `BaseCalculoUnificadaNfseService`: bases RG e SN da NFS-e com documentos de
//! ajuste e locação de imóveis; inconsistências de negócio voltam em `erros`

use crate::decimal::{BigDecimal, RoundingMode};
use crate::erro::{Classe, Falha, Resultado};
use crate::modelo::base_calculo::{BaseUnificadaNfseEntrada, DocumentoAjusteNfse, LocacaoNfse};
use crate::saida_json;
use crate::util::arredondamento::{dividir_por_cem, formatar_moeda, multiplicar, subtrair};

const ANO_EXTINCAO_PIS_COFINS: i32 = 2027;
const ANO_EXTINCAO_ISSQN: i32 = 2033;
const ESCALA_SAIDA: i64 = 2;
const TIPO_OUTROS: &str = "199";

/// `TipoAjusteNfse`: (código, repercute IBS/CBS, repercute SN, requer descrição)
const TIPOS_AJUSTE: [(&str, bool, bool, bool); 7] = [
    ("101", true, false, false),
    ("102", true, false, false),
    ("103", true, false, false),
    ("104", true, false, false),
    ("105", true, false, false),
    (TIPO_OUTROS, true, false, true),
    ("9", false, true, false),
];

#[derive(Debug, Clone, Copy)]
struct TipoAjuste {
    repercute_ibs_cbs: bool,
    repercute_sn: bool,
    requer_descricao: bool,
}

fn tipo_ajuste(codigo: Option<&str>) -> Option<TipoAjuste> {
    let codigo = codigo?;
    TIPOS_AJUSTE
        .iter()
        .find(|(c, ..)| *c == codigo)
        .map(|&(_, ibs_cbs, sn, descricao)| TipoAjuste {
            repercute_ibs_cbs: ibs_cbs,
            repercute_sn: sn,
            requer_descricao: descricao,
        })
}

#[derive(Debug, Clone)]
pub struct ErroBaseNfse {
    pub codigo: Option<String>,
    pub mensagem: Option<String>,
}

saida_json!(ErroBaseNfse {
    "codigo" => codigo: texto,
    "mensagem" => mensagem: texto,
});

fn erro(codigo: &str, mensagem: impl Into<String>) -> ErroBaseNfse {
    ErroBaseNfse {
        codigo: Some(codigo.to_string()),
        mensagem: Some(mensagem.into()),
    }
}

#[derive(Debug, Clone)]
pub struct BaseUnificadaNfseSaida {
    pub base_rg: Option<BigDecimal>,
    pub base_sn: Option<BigDecimal>,
    pub avisos: Option<Vec<String>>,
    pub erros: Option<Vec<ErroBaseNfse>>,
}

saida_json!(BaseUnificadaNfseSaida {
    "baseRG" => base_rg: decimal_sempre,
    "baseSN" => base_sn: decimal_sempre,
    "avisos" => avisos: lista,
    "erros" => erros: lista,
});

fn zero_se_nulo(v: &Option<BigDecimal>) -> BigDecimal {
    v.clone().unwrap_or_else(BigDecimal::zero)
}

fn obrigatorio(v: &Option<BigDecimal>) -> Resultado<&BigDecimal> {
    v.as_ref().ok_or_else(Falha::sem_mensagem)
}

fn documento_nulo() -> Falha {
    Falha::sistema(
        "Cannot invoke \"br.gov.serpro.rtc.api.model.input.basecalculo.DocumentoAjusteNfseInput.getTpAjusteBC()\" because \"doc\" is null",
    )
}

/// separa os documentos aproveitáveis, registrando avisos e erros de leiaute
fn documentos_validos<'a>(
    documentos: &[&'a DocumentoAjusteNfse],
    avisos: &mut Vec<String>,
    erros: &mut Vec<ErroBaseNfse>,
) -> Vec<(&'a DocumentoAjusteNfse, TipoAjuste)> {
    let mut validos = Vec::new();
    for doc in documentos {
        let Some(tipo) = tipo_ajuste(doc.tp_ajuste_bc.as_deref()) else {
            let codigo = doc.tp_ajuste_bc.as_deref().unwrap_or("null");
            avisos.push(format!(
                "Tipo de ajuste '{codigo}' desconhecido: documento ignorado"
            ));
            continue;
        };
        let sem_descricao = doc
            .x_tp_ajuste_bc
            .as_deref()
            .is_none_or(|x| x.trim().is_empty());
        if tipo.requer_descricao && sem_descricao {
            erros.push(erro(
                "LEIAUTE-XTP",
                format!("Campo xTpAjusteBC obrigatório para tipo de ajuste {TIPO_OUTROS}"),
            ));
            continue;
        }
        validos.push((*doc, tipo));
    }
    validos
}

fn somar_ajustes(
    validos: &[(&DocumentoAjusteNfse, TipoAjuste)],
    filtro: fn(&TipoAjuste) -> bool,
) -> Resultado<BigDecimal> {
    let mut total = BigDecimal::zero();
    for (doc, _) in validos.iter().filter(|(_, t)| filtro(t)) {
        total = total.add(obrigatorio(&doc.v_ajuste_aplic)?);
    }
    Ok(total)
}

fn validar_negocio(e: &BaseUnificadaNfseEntrada, erros: &mut Vec<ErroBaseNfse>, locacao: bool) {
    let Some(op) = e.op_simp_nac else { return };
    if op.exige_regime_apuracao() && e.reg_ap.is_none() {
        erros.push(erro(
            "EXXX-REGAP",
            "Campo regApIBSCBSSN obrigatório para optante do Simples Nacional",
        ));
    }
    if op.proibe_regime_apuracao() && e.reg_ap.is_some() {
        erros.push(erro(
            "EXXX-REGAP",
            "Campo regApIBSCBSSN proibido para não optante do Simples Nacional",
        ));
    }
    if op.veda_locacao() && locacao {
        erros.push(erro(
            "EXXX-LOC-SN",
            "Locação de imóveis incompatível com optante do Simples Nacional",
        ));
    }
}

/// valor efetivo da locação: fator de copropriedade aplicado a cada parcela
struct LocacaoEfetiva {
    v_serv: BigDecimal,
    v_desc_incond: BigDecimal,
    ajustes: BigDecimal,
}

fn locacao_efetiva(l: &LocacaoNfse) -> Resultado<LocacaoEfetiva> {
    let fator = dividir_por_cem(obrigatorio(&l.p_copropriedade)?);
    Ok(LocacaoEfetiva {
        v_serv: multiplicar(&fator, obrigatorio(&l.v_tot_oper)?),
        v_desc_incond: multiplicar(&fator, obrigatorio(&l.v_desc_incond_tot)?),
        ajustes: multiplicar(&fator, obrigatorio(&l.soma_ajustes_loc_imoveis)?),
    })
}

/// PIS/COFINS (e ISSQN fora da locação) só deduzem enquanto existirem
fn deducoes_extintas(e: &BaseUnificadaNfseEntrada, ano: i32, com_issqn: bool) -> Vec<BigDecimal> {
    let vigente = |v: &Option<BigDecimal>, extincao: i32| {
        if ano < extincao {
            zero_se_nulo(v)
        } else {
            BigDecimal::zero()
        }
    };
    let mut deducoes = Vec::new();
    if com_issqn {
        deducoes.push(vigente(&e.v_issqn, ANO_EXTINCAO_ISSQN));
    }
    deducoes.push(vigente(&e.v_pis, ANO_EXTINCAO_PIS_COFINS));
    deducoes.push(vigente(&e.v_cofins, ANO_EXTINCAO_PIS_COFINS));
    deducoes
}

fn subtrair_todos(inicial: BigDecimal, parcelas: &[BigDecimal]) -> BigDecimal {
    parcelas.iter().fold(inicial, |acc, p| subtrair(&acc, p))
}

fn base_rg(
    e: &BaseUnificadaNfseEntrada,
    ano: i32,
    ajuste_ibs_cbs: &BigDecimal,
) -> Resultado<BigDecimal> {
    if let (true, Some(l)) = (e.situacao.is_some_and(|s| s.eh_locacao()), &e.locacao) {
        let loc = locacao_efetiva(l)?;
        let mut parcelas = vec![loc.v_desc_incond, loc.ajustes];
        parcelas.extend(deducoes_extintas(e, ano, false));
        return Ok(subtrair_todos(loc.v_serv, &parcelas));
    }
    let mut parcelas = vec![zero_se_nulo(&e.v_desc_incond), ajuste_ibs_cbs.clone()];
    parcelas.extend(deducoes_extintas(e, ano, true));
    Ok(subtrair_todos(zero_se_nulo(&e.v_serv), &parcelas))
}

fn base_sn(
    e: &BaseUnificadaNfseEntrada,
    ajuste_ibs_cbs: &BigDecimal,
    ajuste_parceiro: &BigDecimal,
) -> Resultado<Option<BigDecimal>> {
    let (Some(op), Some(reg_ap), Some(situacao)) = (e.op_simp_nac, e.reg_ap, e.situacao) else {
        return Ok(None);
    };
    if op.base_sn_inaplicavel() {
        return Ok(None);
    }
    let mut base = match (situacao.eh_locacao(), &e.locacao) {
        (true, Some(l)) => {
            let loc = locacao_efetiva(l)?;
            subtrair(&loc.v_serv, &loc.v_desc_incond)
        }
        _ => subtrair(&zero_se_nulo(&e.v_serv), &zero_se_nulo(&e.v_desc_incond)),
    };
    base = subtrair(&base, ajuste_ibs_cbs);
    if situacao.eh_salao_parceiro() && reg_ap.calcula_base_sn() {
        base = subtrair(&base, ajuste_parceiro);
    }
    Ok(Some(base))
}

fn escala_saida(v: Option<BigDecimal>) -> Resultado<Option<BigDecimal>> {
    v.map(|b| b.set_scale(ESCALA_SAIDA, RoundingMode::HalfEven))
        .transpose()
        .map_err(|e| Falha::sistema(e.to_string()))
}

fn erro_base_negativa(rotulo: &str, base: &Option<BigDecimal>) -> Option<ErroBaseNfse> {
    let b = base.as_ref().filter(|b| b.signum() < 0)?;
    Some(erro(
        "BASE-NEGATIVA",
        format!("Base de cálculo {rotulo} negativa: {}", formatar_moeda(b)),
    ))
}

pub fn calcular_unificada_nfse(e: &BaseUnificadaNfseEntrada) -> Resultado<BaseUnificadaNfseSaida> {
    let (Some(op), Some(situacao), Some(ano)) = (e.op_simp_nac, e.situacao, e.ano_fato_gerador)
    else {
        return Err(Falha::sem_mensagem());
    };
    let mut avisos = Vec::new();
    let mut erros = Vec::new();
    let documentos: Vec<&DocumentoAjusteNfse> = e
        .documentos
        .iter()
        .flatten()
        .map(|d| d.as_ref().ok_or_else(documento_nulo))
        .collect::<Resultado<_>>()?;
    if situacao.eh_locacao() && e.locacao.is_none() {
        return Err(Falha::new(
            Classe::CampoInvalido,
            "Campo locacao obrigatório quando situacao é locacao9903",
        ));
    }
    validar_negocio(e, &mut erros, situacao.eh_locacao());
    let validos = documentos_validos(&documentos, &mut avisos, &mut erros);
    if op.eh_mei() {
        avisos.push("E1302: Operação MEI: bases de cálculo não aplicáveis".into());
        if !documentos.is_empty() {
            erros.push(erro(
                "E0436",
                "Documentos de ajuste não são permitidos para MEI",
            ));
        }
        return Ok(BaseUnificadaNfseSaida {
            base_rg: None,
            base_sn: None,
            avisos: Some(avisos),
            erros: Some(erros),
        });
    }
    let (ajuste_ibs_cbs, ajuste_parceiro) = if situacao.eh_locacao() {
        (BigDecimal::zero(), BigDecimal::zero())
    } else {
        (
            somar_ajustes(&validos, |t| t.repercute_ibs_cbs)?,
            somar_ajustes(&validos, |t| t.repercute_sn)?,
        )
    };
    let v_serv = zero_se_nulo(&e.v_serv);
    if ajuste_ibs_cbs.signum() > 0 && ajuste_ibs_cbs.compare_to(&v_serv).is_ge() {
        erros.push(erro(
            "E1534",
            format!(
                "Soma dos ajustes ({}) excede ou iguala o valor do serviço ({})",
                formatar_moeda(&ajuste_ibs_cbs),
                formatar_moeda(&v_serv)
            ),
        ));
    }
    if situacao.eh_locacao() && !documentos.is_empty() {
        avisos.push("Documentos de ajuste ignorados em contexto de locação de imóveis".into());
    }
    if !situacao.eh_locacao() && e.locacao.is_some() {
        avisos.push("Grupo locacao ignorado fora de contexto de locação de imóveis".into());
    }
    let rg = base_rg(e, ano, &ajuste_ibs_cbs)?;
    let sn = base_sn(e, &ajuste_ibs_cbs, &ajuste_parceiro)?;
    if op.eh_pendente() {
        avisos.push("Optante pendente: cálculo realizado com ressalva".into());
    }
    let rg = escala_saida(Some(rg))?;
    let sn = escala_saida(sn)?;
    erros.extend(erro_base_negativa("RG", &rg));
    erros.extend(erro_base_negativa("SN", &sn));
    Ok(BaseUnificadaNfseSaida {
        base_rg: rg,
        base_sn: sn,
        avisos: Some(avisos),
        erros: Some(erros),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::ler_documento;

    fn calcular(json: &str) -> Option<BaseUnificadaNfseSaida> {
        let e: BaseUnificadaNfseEntrada = ler_documento(json.as_bytes()).ok().flatten()?;
        calcular_unificada_nfse(&e).ok()
    }

    fn texto(v: &Option<BigDecimal>) -> Option<String> {
        v.as_ref().map(ToString::to_string)
    }

    #[test]
    fn comum_com_documentos() {
        let s = calcular(
            r#"{"anoFatoGerador":2026,"opSimpNac":1,"situacao":"comum","vServ":1000,"vDescIncond":10,"vISSQN":20,"vPIS":5,"vCOFINS":7,
            "documentos":[{"tpAjusteBC":"101","vAjusteAplic":10},{"tpAjusteBC":"777","vAjusteAplic":1},{"tpAjusteBC":"199","vAjusteAplic":1}]}"#,
        );
        let s = s.as_ref();
        assert_eq!(s.and_then(|s| texto(&s.base_rg)).as_deref(), Some("948.00"));
        assert_eq!(s.and_then(|s| s.avisos.as_ref()).map(Vec::len), Some(1));
        assert_eq!(s.and_then(|s| s.erros.as_ref()).map(Vec::len), Some(1));
    }

    #[test]
    fn locacao_aplica_copropriedade() {
        let s = calcular(
            r#"{"anoFatoGerador":2026,"opSimpNac":4,"situacao":"locacao9903","regApIBSCBSSN":2,
            "locacao":{"pCopropriedade":50,"vTotOper":1000,"vDescIncondTot":100,"somaAjustesLocImoveis":20}}"#,
        );
        assert_eq!(
            s.as_ref().and_then(|s| texto(&s.base_rg)).as_deref(),
            Some("440.00")
        );
        assert_eq!(
            s.as_ref().and_then(|s| texto(&s.base_sn)).as_deref(),
            Some("450.00")
        );
    }

    #[test]
    fn mei_sem_bases() {
        let s = calcular(r#"{"anoFatoGerador":2026,"opSimpNac":2,"situacao":"comum","vServ":100}"#);
        assert!(s.is_some_and(|s| s.base_rg.is_none() && s.base_sn.is_none()));
    }
}
