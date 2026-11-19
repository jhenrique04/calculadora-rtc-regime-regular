//! DTOs de entrada da calculadora (`api.model.input`), com a desserialização do Jackson

use chrono::NaiveDate;

use crate::decimal::BigDecimal;
use crate::erro::{Classe, Falha, Resultado};
use crate::json::{DataHora, Leitor, LerJson, R, ler_enum_codigo};
use crate::objeto_json;

const PACOTE: &str = "br.gov.serpro.rtc.api.model.input";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoEnteGov {
    Uniao,
    Estado,
    DistritoFederal,
    Municipio,
    ConsorcioPublico,
    ComiteGestorIbs,
}

impl TipoEnteGov {
    const VALORES: [(&'static str, Self); 6] = [
        ("1", Self::Uniao),
        ("2", Self::Estado),
        ("3", Self::DistritoFederal),
        ("4", Self::Municipio),
        ("5", Self::ConsorcioPublico),
        ("6", Self::ComiteGestorIbs),
    ];

    pub fn codigo(self) -> u8 {
        match self {
            Self::Uniao => 1,
            Self::Estado => 2,
            Self::DistritoFederal => 3,
            Self::Municipio => 4,
            Self::ConsorcioPublico => 5,
            Self::ComiteGestorIbs => 6,
        }
    }

    fn equivalente(self) -> Self {
        match self {
            Self::DistritoFederal => Self::Estado,
            Self::ConsorcioPublico | Self::ComiteGestorIbs => Self::Municipio,
            outro => outro,
        }
    }

    pub fn eh_uniao(self) -> bool {
        self.equivalente() == Self::Uniao
    }

    pub fn eh_estado(self) -> bool {
        self.equivalente() == Self::Estado
    }

    pub fn eh_municipio(self) -> bool {
        self.equivalente() == Self::Municipio
    }
}

impl LerJson for TipoEnteGov {
    fn tipo_java() -> String {
        format!("{PACOTE}.calculadora.enumeration.TipoEnteGovernamental")
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        ler_enum_codigo(l, &Self::tipo_java(), &Self::VALORES)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TipoOperacaoGov(pub u8);

impl LerJson for TipoOperacaoGov {
    fn tipo_java() -> String {
        format!("{PACOTE}.calculadora.enumeration.TipoOperacaoGovernamental")
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        let valores = [
            ("1", Self(1)),
            ("2", Self(2)),
            ("3", Self(3)),
            ("4", Self(4)),
            ("5", Self(5)),
        ];
        ler_enum_codigo(l, &Self::tipo_java(), &valores)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompraGov {
    pub tp_ente_gov: Option<TipoEnteGov>,
    pub tp_oper_gov: Option<TipoOperacaoGov>,
}

objeto_json!(CompraGov, "br.gov.serpro.rtc.api.model.input.CompraGovernamentalInput", {
    "tpEnteGov" => tp_ente_gov: TipoEnteGov,
    "tpOperGov" => tp_oper_gov: TipoOperacaoGov,
});

#[derive(Debug, Clone, Default)]
pub struct ImpostoSeletivoEntrada {
    pub cst: Option<String>,
    pub c_class_trib: Option<String>,
    pub base_calculo: Option<BigDecimal>,
    pub quantidade: Option<BigDecimal>,
    pub unidade: Option<String>,
    pub imposto_informado: Option<BigDecimal>,
}

impl ImpostoSeletivoEntrada {
    pub fn valor_informado(&self) -> BigDecimal {
        self.imposto_informado
            .clone()
            .unwrap_or_else(BigDecimal::zero)
    }
}

objeto_json!(ImpostoSeletivoEntrada, "br.gov.serpro.rtc.api.model.input.ImpostoSeletivoInput", {
    "cst" => cst: String,
    "cClassTrib" => c_class_trib: String,
    "baseCalculo" => base_calculo: BigDecimal,
    "quantidade" => quantidade: BigDecimal,
    "unidade" => unidade: String,
    "impostoInformado" => imposto_informado: BigDecimal,
});

#[derive(Debug, Clone, Default)]
pub struct TributacaoRegularEntrada {
    pub cst: Option<String>,
    pub c_class_trib: Option<String>,
}

objeto_json!(TributacaoRegularEntrada, "br.gov.serpro.rtc.api.model.input.TributacaoRegularInput", {
    "cst" => cst: String,
    "cClassTrib" => c_class_trib: String,
});

#[derive(Debug, Clone, Default)]
pub struct AliquotasNominaisIs {
    pub ad_valorem: Option<BigDecimal>,
    pub ad_rem: Option<BigDecimal>,
}

objeto_json!(AliquotasNominaisIs, "br.gov.serpro.rtc.api.model.input.AliquotasNominaisImpostoSeletivo", {
    "adValorem" => ad_valorem: BigDecimal,
    "adRem" => ad_rem: BigDecimal,
});

#[derive(Debug, Clone, Default)]
pub struct AliquotasNominais {
    pub cbs: Option<BigDecimal>,
    pub ibs_estadual: Option<BigDecimal>,
    pub ibs_municipal: Option<BigDecimal>,
    pub imposto_seletivo: Option<AliquotasNominaisIs>,
}

objeto_json!(AliquotasNominais, "br.gov.serpro.rtc.api.model.input.AliquotasNominaisInput", {
    "cbs" => cbs: BigDecimal,
    "ibsEstadual" => ibs_estadual: BigDecimal,
    "ibsMunicipal" => ibs_municipal: BigDecimal,
    "impostoSeletivo" => imposto_seletivo: AliquotasNominaisIs,
});

/// campos de `ComponentesBaseCalculoInput`, na ordem de `ElementoBaseCalculo`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementoBc {
    VProd,
    VFrete,
    VSeg,
    VOutro,
    VIi,
    VIs,
    VDesc,
    VPis,
    VCofins,
    VIcms,
    VIcmsUfDest,
    VFcp,
    VFcpUfDest,
    VIcmsMono,
    VIssqn,
}

impl ElementoBc {
    pub const TODOS: [Self; 15] = [
        Self::VProd,
        Self::VFrete,
        Self::VSeg,
        Self::VOutro,
        Self::VIi,
        Self::VIs,
        Self::VDesc,
        Self::VPis,
        Self::VCofins,
        Self::VIcms,
        Self::VIcmsUfDest,
        Self::VFcp,
        Self::VFcpUfDest,
        Self::VIcmsMono,
        Self::VIssqn,
    ];

    pub fn tag(self) -> &'static str {
        match self {
            Self::VProd => "vProd",
            Self::VFrete => "vFrete",
            Self::VSeg => "vSeg",
            Self::VOutro => "vOutro",
            Self::VIi => "vII",
            Self::VIs => "vIS",
            Self::VDesc => "vDesc",
            Self::VPis => "vPIS",
            Self::VCofins => "vCOFINS",
            Self::VIcms => "vICMS",
            Self::VIcmsUfDest => "vICMSUFDest",
            Self::VFcp => "vFCP",
            Self::VFcpUfDest => "vFCPUFDest",
            Self::VIcmsMono => "vICMSMono",
            Self::VIssqn => "vISSQN",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ComponentesBc {
    pub valores: [Option<BigDecimal>; 15],
}

impl ComponentesBc {
    pub fn valor(&self, e: ElementoBc) -> Option<&BigDecimal> {
        self.valores[e as usize].as_ref()
    }

    pub fn possui_componentes(&self) -> bool {
        self.valores.iter().any(Option::is_some)
    }
}

impl LerJson for ComponentesBc {
    fn tipo_java() -> String {
        format!("{PACOTE}.basecalculo.ComponentesBaseCalculoInput")
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        crate::json::ler_objeto(l, &Self::tipo_java(), |obj: &mut Self, nome, l| {
            match ElementoBc::TODOS.iter().find(|e| e.tag() == nome) {
                Some(e) => {
                    obj.valores[*e as usize] = BigDecimal::ler(l)?;
                    Ok(true)
                }
                None => Ok(false),
            }
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    pub numero: Option<i32>,
    pub ncm: Option<String>,
    pub nbs: Option<String>,
    pub cst: Option<String>,
    pub c_class_trib: Option<String>,
    pub base_calculo: Option<BigDecimal>,
    pub quantidade: Option<BigDecimal>,
    pub unidade: Option<String>,
    pub imposto_seletivo: Option<ImpostoSeletivoEntrada>,
    pub tributacao_regular: Option<TributacaoRegularEntrada>,
    pub componentes_bc: Option<ComponentesBc>,
    pub aliquotas_nominais: Option<AliquotasNominais>,
}

objeto_json!(Item, "br.gov.serpro.rtc.api.model.input.ItemOperacaoInput", {
    "numero" => numero: i32,
    "ncm" => ncm: String,
    "nbs" => nbs: String,
    "cst" => cst: String,
    "cClassTrib" => c_class_trib: String,
    "baseCalculo" => base_calculo: BigDecimal,
    "quantidade" => quantidade: BigDecimal,
    "unidade" => unidade: String,
    "impostoSeletivo" => imposto_seletivo: ImpostoSeletivoEntrada,
    "tributacaoRegular" => tributacao_regular: TributacaoRegularEntrada,
    "gComponentesBC" => componentes_bc: ComponentesBc,
    "aliquotasNominais" => aliquotas_nominais: AliquotasNominais,
});

#[derive(Debug, Clone, Default)]
pub struct Operacao {
    pub id: Option<String>,
    pub versao: Option<String>,
    pub dh_fato_gerador: Option<DataHora>,
    pub data_hora_emissao: Option<DataHora>,
    pub municipio: Option<i64>,
    pub uf: Option<String>,
    pub tp_doc: Option<i32>,
    pub compra_gov: Option<CompraGov>,
    pub itens: Option<Vec<Option<Item>>>,
}

objeto_json!(Operacao, "br.gov.serpro.rtc.api.model.input.OperacaoInput", {
    "id" => id: String,
    "versao" => versao: String,
    "dhFatoGerador" => dh_fato_gerador: DataHora,
    "dataHoraEmissao" => data_hora_emissao: DataHora,
    "municipio" => municipio: i64,
    "uf" => uf: String,
    "tpDoc" => tp_doc: i32,
    "gCompraGov" => compra_gov: CompraGov,
    "itens" => itens: Vec<Option<Item>>,
});

impl Operacao {
    pub fn tp_ente_gov(&self) -> Option<TipoEnteGov> {
        self.compra_gov.as_ref().and_then(|c| c.tp_ente_gov)
    }

    pub fn fato_gerador(&self) -> Resultado<NaiveDate> {
        self.dh_fato_gerador
            .or(self.data_hora_emissao)
            .map(|d| d.data())
            .ok_or_else(|| {
                Falha::new(
                    Classe::DataFatoGeradorNaoInformada,
                    "A data do fato gerador não foi informada.",
                )
            })
    }
}
