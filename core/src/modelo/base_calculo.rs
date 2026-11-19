//! entradas do `BaseCalculoController`

use std::marker::PhantomData;

use crate::decimal::BigDecimal;
use crate::json::{
    ErroJson, Leitor, LerJson, R, Token, ler_enum_criador_int, ler_enum_nome, ler_objeto,
    token_inesperado,
};
use crate::modelo::entrada::{ComponentesBc, ElementoBc};
use crate::objeto_json;
use crate::servico::base_calculo::Natureza;

const PACOTE: &str = "br.gov.serpro.rtc.api.model.input.basecalculo";

impl Natureza {
    /// `BaseCalculoNatureza.fromString`: valor desconhecido vira `null`
    fn de_texto(valor: &str) -> Option<Self> {
        if valor.trim().is_empty() {
            return None;
        }
        let normalizado: String = valor
            .to_uppercase()
            .chars()
            .map(|c| match c {
                'Á' | 'À' | 'Ã' => 'A',
                'Ç' => 'C',
                'É' | 'Ê' => 'E',
                'Í' => 'I',
                'Ó' | 'Ô' | 'Õ' => 'O',
                'Ú' => 'U',
                ' ' => '_',
                outro => outro,
            })
            .collect();
        match normalizado.as_str() {
            "BEM" => Some(Self::Bem),
            "SERVICO" => Some(Self::Servico),
            _ => None,
        }
    }
}

impl LerJson for Natureza {
    fn tipo_java() -> String {
        format!("{PACOTE}.enumeration.BaseCalculoNatureza")
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        let tipo = Self::tipo_java();
        let t = l
            .atual()
            .cloned()
            .ok_or_else(|| ErroJson::new("Unexpected end-of-input"))?;
        match &t {
            Token::Nulo => Ok(None),
            Token::InicioLista => Err(ErroJson::new(format!(
                "Input mismatch reading Enum `{tipo}`: properties-based `@JsonCreator` ([method {tipo}#fromString(java.lang.String)]) expects String Value, got Array value (`JsonToken.START_ARRAY`)"
            ))),
            Token::InicioObjeto => token_inesperado(&tipo, &t),
            _ => Ok(Self::de_texto(&t.texto())),
        }
    }
}

/// `BaseCalculoUnificadaInput`: os componentes vêm achatados (`@JsonUnwrapped`)
#[derive(Debug, Clone, Default)]
pub struct BaseUnificadaEntrada {
    pub ano_fato_gerador: Option<i32>,
    pub tipo_documento: Option<i32>,
    pub natureza: Option<Natureza>,
    pub componentes: ComponentesBc,
}

impl LerJson for BaseUnificadaEntrada {
    fn tipo_java() -> String {
        format!("{PACOTE}.BaseCalculoUnificadaInput")
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        ler_objeto(l, &Self::tipo_java(), |obj: &mut Self, nome, l| {
            match nome {
                "anoFatoGerador" => obj.ano_fato_gerador = i32::ler(l)?,
                "tipoDocumento" => obj.tipo_documento = i32::ler(l)?,
                "natureza" => obj.natureza = Natureza::ler(l)?,
                _ => match ElementoBc::TODOS.iter().find(|e| e.tag() == nome) {
                    Some(e) => obj.componentes.valores[*e as usize] = BigDecimal::ler(l)?,
                    None => return Ok(false),
                },
            }
            Ok(true)
        })
    }
}

/// grupo de campos que deixa de existir a partir de um ano (rótulo, campo)
pub struct GrupoExtinto {
    pub ano: i32,
    pub campos: &'static [(&'static str, &'static str)],
}

/// diferença entre `BaseCalculoISMercadoriasInput` e `BaseCalculoCibsInput`
pub trait LayoutMercadorias {
    const CLASSE: &'static str;
    const ANO_INICIO: i32;
    const MENSAGEM_ANO: &'static str;
    const EXTINTOS: &'static [GrupoExtinto];
    const INTEGRA: &'static [&'static str];
    const NAO_INTEGRA: &'static [&'static str];
    /// ordem de declaração dos campos na classe Java
    const CAMPOS: &'static [&'static str];
}

pub struct IsMercadorias;
pub struct CibsMercadorias;

impl LayoutMercadorias for IsMercadorias {
    const CLASSE: &'static str = "BaseCalculoISMercadoriasInput";
    const ANO_INICIO: i32 = 2027;
    const MENSAGEM_ANO: &'static str = "Para IS, o ano do fato gerador deve ser 2027 ou superior";
    const EXTINTOS: &'static [GrupoExtinto] = &[GrupoExtinto {
        ano: 2033,
        campos: &[("ICMS", "icms"), ("ISS", "iss")],
    }];
    const INTEGRA: &'static [&'static str] = &[
        "valorBem",
        "ajusteAcrescimos",
        "juros",
        "multas",
        "encargos",
        "freteCobrado",
        "outrosTributos",
        "demaisImportancias",
    ];
    const NAO_INTEGRA: &'static [&'static str] = &[
        "cosip",
        "ipi",
        "descontoIncondicional",
        "bonificacao",
        "devolucaoVendas",
        "icms",
        "iss",
    ];
    const CAMPOS: &'static [&'static str] = &[
        "valorBem",
        "ajusteAcrescimos",
        "juros",
        "multas",
        "encargos",
        "freteCobrado",
        "outrosTributos",
        "demaisImportancias",
        "icms",
        "iss",
        "cosip",
        "ipi",
        "descontoIncondicional",
        "bonificacao",
        "devolucaoVendas",
    ];
}

impl LayoutMercadorias for CibsMercadorias {
    const CLASSE: &'static str = "BaseCalculoCibsInput";
    const ANO_INICIO: i32 = 2026;
    const MENSAGEM_ANO: &'static str =
        "Para CBS/IBS, o ano do fato gerador deve ser 2026 ou superior";
    const EXTINTOS: &'static [GrupoExtinto] = &[
        GrupoExtinto {
            ano: 2027,
            campos: &[
                ("PIS", "pis"),
                ("PIS Importação", "pisImportacao"),
                ("COFINS", "cofins"),
                ("COFINS Importação", "cofinsImportacao"),
            ],
        },
        GrupoExtinto {
            ano: 2033,
            campos: &[("ICMS", "icms"), ("ISS", "iss")],
        },
    ];
    const INTEGRA: &'static [&'static str] = &[
        "valorBem",
        "ajusteAcrescimos",
        "juros",
        "multas",
        "encargos",
        "frete",
        "impostoSeletivo",
        "outrosTributos",
        "demaisImportancias",
    ];
    const NAO_INTEGRA: &'static [&'static str] = &[
        "cosip",
        "ipi",
        "descontoIncondicional",
        "pis",
        "pisImportacao",
        "cofins",
        "cofinsImportacao",
        "icms",
        "iss",
    ];
    const CAMPOS: &'static [&'static str] = &[
        "valorBem",
        "ajusteAcrescimos",
        "juros",
        "multas",
        "encargos",
        "frete",
        "impostoSeletivo",
        "outrosTributos",
        "demaisImportancias",
        "icms",
        "iss",
        "pis",
        "pisImportacao",
        "cofins",
        "cofinsImportacao",
        "cosip",
        "ipi",
        "descontoIncondicional",
    ];
}

/// entrada de base de mercadorias: valores na ordem de `L::CAMPOS`, começando
/// em zero; `null` explícito sobrescreve o padrão
#[derive(Debug, Clone)]
pub struct MercadoriasEntrada<L> {
    pub ano_fato_gerador: Option<i32>,
    pub valores: Vec<Option<BigDecimal>>,
    layout: PhantomData<L>,
}

impl<L: LayoutMercadorias> Default for MercadoriasEntrada<L> {
    fn default() -> Self {
        Self {
            ano_fato_gerador: None,
            valores: vec![Some(BigDecimal::zero()); L::CAMPOS.len()],
            layout: PhantomData,
        }
    }
}

impl<L: LayoutMercadorias> MercadoriasEntrada<L> {
    fn indice(campo: &str) -> Option<usize> {
        L::CAMPOS.iter().position(|c| *c == campo)
    }

    pub fn valor(&self, campo: &str) -> &Option<BigDecimal> {
        const AUSENTE: Option<BigDecimal> = None;
        Self::indice(campo)
            .and_then(|i| self.valores.get(i))
            .unwrap_or(&AUSENTE)
    }
}

impl<L: LayoutMercadorias> LerJson for MercadoriasEntrada<L> {
    fn tipo_java() -> String {
        format!("{PACOTE}.{}", L::CLASSE)
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        ler_objeto(l, &Self::tipo_java(), |obj: &mut Self, nome, l| {
            if nome == "anoFatoGerador" {
                obj.ano_fato_gerador = i32::ler(l)?;
                return Ok(true);
            }
            match Self::indice(nome) {
                Some(i) => {
                    let v = BigDecimal::ler(l)?;
                    if let Some(slot) = obj.valores.get_mut(i) {
                        *slot = v;
                    }
                    Ok(true)
                }
                None => Ok(false),
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpcaoSimplesNacional {
    NaoOptante,
    Mei,
    OptanteMeEpp,
    OptantePendente,
}

impl OpcaoSimplesNacional {
    const VALORES: [(i32, Self); 4] = [
        (1, Self::NaoOptante),
        (2, Self::Mei),
        (3, Self::OptanteMeEpp),
        (4, Self::OptantePendente),
    ];

    pub fn eh_mei(self) -> bool {
        self == Self::Mei
    }

    pub fn eh_pendente(self) -> bool {
        self == Self::OptantePendente
    }

    pub fn exige_regime_apuracao(self) -> bool {
        matches!(self, Self::OptanteMeEpp | Self::OptantePendente)
    }

    pub fn proibe_regime_apuracao(self) -> bool {
        self == Self::NaoOptante
    }

    pub fn veda_locacao(self) -> bool {
        matches!(self, Self::Mei | Self::OptanteMeEpp)
    }

    pub fn base_sn_inaplicavel(self) -> bool {
        matches!(self, Self::NaoOptante | Self::Mei)
    }
}

impl LerJson for OpcaoSimplesNacional {
    fn tipo_java() -> String {
        format!("{PACOTE}.enumeration.OpcaoSimplesNacional")
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        ler_enum_criador_int(l, &Self::tipo_java(), &Self::VALORES)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegimeApuracaoSn {
    IbsCbsPeloSn,
    CbsPeloSnIbsRegular,
    AmbosRegulares,
}

impl RegimeApuracaoSn {
    const VALORES: [(i32, Self); 3] = [
        (1, Self::IbsCbsPeloSn),
        (2, Self::CbsPeloSnIbsRegular),
        (3, Self::AmbosRegulares),
    ];

    pub fn calcula_base_sn(self) -> bool {
        self != Self::AmbosRegulares
    }
}

impl LerJson for RegimeApuracaoSn {
    fn tipo_java() -> String {
        format!("{PACOTE}.enumeration.RegimeApuracaoIbsCbsSN")
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        ler_enum_criador_int(l, &Self::tipo_java(), &Self::VALORES)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SituacaoOperacaoNfse {
    Comum,
    SalaoParceiro,
    Locacao,
}

impl SituacaoOperacaoNfse {
    const VALORES: [(&'static str, Self); 3] = [
        ("comum", Self::Comum),
        ("salaoParceiro", Self::SalaoParceiro),
        ("locacao9903", Self::Locacao),
    ];

    pub fn eh_locacao(self) -> bool {
        self == Self::Locacao
    }

    pub fn eh_salao_parceiro(self) -> bool {
        self == Self::SalaoParceiro
    }
}

impl LerJson for SituacaoOperacaoNfse {
    fn tipo_java() -> String {
        format!("{PACOTE}.enumeration.SituacaoOperacaoNfse")
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        ler_enum_nome(l, &Self::tipo_java(), &Self::VALORES)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DocumentoAjusteNfse {
    pub tp_ajuste_bc: Option<String>,
    pub v_ajuste_aplic: Option<BigDecimal>,
    pub x_tp_ajuste_bc: Option<String>,
}

objeto_json!(DocumentoAjusteNfse, "br.gov.serpro.rtc.api.model.input.basecalculo.DocumentoAjusteNfseInput", {
    "tpAjusteBC" => tp_ajuste_bc: String,
    "vAjusteAplic" => v_ajuste_aplic: BigDecimal,
    "xTpAjusteBC" => x_tp_ajuste_bc: String,
});

#[derive(Debug, Clone, Default)]
pub struct LocacaoNfse {
    pub p_copropriedade: Option<BigDecimal>,
    pub v_tot_oper: Option<BigDecimal>,
    pub v_desc_incond_tot: Option<BigDecimal>,
    pub soma_ajustes_loc_imoveis: Option<BigDecimal>,
}

objeto_json!(LocacaoNfse, "br.gov.serpro.rtc.api.model.input.basecalculo.LocacaoNfseInput", {
    "pCopropriedade" => p_copropriedade: BigDecimal,
    "vTotOper" => v_tot_oper: BigDecimal,
    "vDescIncondTot" => v_desc_incond_tot: BigDecimal,
    "somaAjustesLocImoveis" => soma_ajustes_loc_imoveis: BigDecimal,
});

#[derive(Debug, Clone, Default)]
pub struct BaseUnificadaNfseEntrada {
    pub ano_fato_gerador: Option<i32>,
    pub op_simp_nac: Option<OpcaoSimplesNacional>,
    pub situacao: Option<SituacaoOperacaoNfse>,
    pub v_serv: Option<BigDecimal>,
    pub v_desc_incond: Option<BigDecimal>,
    pub v_issqn: Option<BigDecimal>,
    pub v_pis: Option<BigDecimal>,
    pub v_cofins: Option<BigDecimal>,
    pub reg_ap: Option<RegimeApuracaoSn>,
    pub documentos: Option<Vec<Option<DocumentoAjusteNfse>>>,
    pub locacao: Option<LocacaoNfse>,
}

objeto_json!(BaseUnificadaNfseEntrada, "br.gov.serpro.rtc.api.model.input.basecalculo.BaseCalculoUnificadaNfseInput", {
    "anoFatoGerador" => ano_fato_gerador: i32,
    "opSimpNac" => op_simp_nac: OpcaoSimplesNacional,
    "situacao" => situacao: SituacaoOperacaoNfse,
    "vServ" => v_serv: BigDecimal,
    "vDescIncond" => v_desc_incond: BigDecimal,
    "vISSQN" => v_issqn: BigDecimal,
    "vPIS" => v_pis: BigDecimal,
    "vCOFINS" => v_cofins: BigDecimal,
    "regApIBSCBSSN" => reg_ap: RegimeApuracaoSn,
    "documentos" => documentos: Vec<Option<DocumentoAjusteNfse>>,
    "locacao" => locacao: LocacaoNfse,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::ler_documento;

    fn ler<T: LerJson>(json: &str) -> Result<Option<T>, String> {
        ler_documento(json.as_bytes()).map_err(|e| e.mensagem)
    }

    #[test]
    fn natureza_normaliza_acentos_e_caixa() {
        assert_eq!(Natureza::de_texto("sErViÇo"), Some(Natureza::Servico));
        assert_eq!(Natureza::de_texto("BEM "), None);
        assert_eq!(Natureza::de_texto(" "), None);
    }

    #[test]
    fn opcao_simples_aceita_texto_e_decimal() {
        assert_eq!(
            ler::<OpcaoSimplesNacional>("\" 2 \""),
            Ok(Some(OpcaoSimplesNacional::Mei))
        );
        assert_eq!(
            ler::<OpcaoSimplesNacional>("2.7"),
            Ok(Some(OpcaoSimplesNacional::Mei))
        );
        assert_eq!(
            ler::<OpcaoSimplesNacional>("\"\""),
            Err("Cannot construct instance of `br.gov.serpro.rtc.api.model.input.basecalculo.enumeration.OpcaoSimplesNacional`, problem: Código inválido para OpcaoSimplesNacional: 0".into())
        );
    }

    #[test]
    fn situacao_por_nome_ou_indice() {
        assert_eq!(
            ler::<SituacaoOperacaoNfse>("1"),
            Ok(Some(SituacaoOperacaoNfse::SalaoParceiro))
        );
        assert_eq!(
            ler::<SituacaoOperacaoNfse>("\" comum\""),
            Ok(Some(SituacaoOperacaoNfse::Comum))
        );
        assert!(
            ler::<SituacaoOperacaoNfse>("5")
                .is_err_and(|e| e.ends_with("index value outside legal index range [0..2]"))
        );
        assert!(
            ler::<SituacaoOperacaoNfse>("\"COMUM\"")
                .is_err_and(|e| e.ends_with("[comum, locacao9903, salaoParceiro]"))
        );
    }

    #[test]
    fn mercadorias_comecam_em_zero() {
        let e = ler::<MercadoriasEntrada<CibsMercadorias>>(
            r#"{"anoFatoGerador":2026,"pis":null,"x":1}"#,
        )
        .ok()
        .flatten()
        .unwrap_or_default();
        assert!(e.valor("pis").is_none());
        assert_eq!(
            e.valor("cofins")
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("0")
        );
    }
}
