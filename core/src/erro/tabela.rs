// gerado de api/exceptionhandler/enumeration/ProblemType.java e da hierarquia de exceções
use super::{Categoria, CodigoErro, EstadoItem, Problema};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classe {
    AliquotaAdRemNaoEncontrada,
    AliquotaImpostoSeletivoNaoInformada,
    AliquotaNaoEncontrada,
    AliquotaNegativa,
    AliquotaPadraoNaoEncontrada,
    AliquotaReferenciaNaoEncontrada,
    AliquotasNominaisInformadasIndevidamente,
    AliquotasNominaisNaoInformadas,
    BaseCalculoComponentesInconsistente,
    BaseCalculoInconsistente,
    BaseCalculoMenorBaseCalculoImpostoSeletivo,
    BaseCalculoNaoInformada,
    CampoInvalido,
    Captcha,
    ClassificacaoTributariaNaoEncontrada,
    ClassificacaoTributariaNaoVinculadaSituacaoTributaria,
    ClassificacaoTributariaNaoVinculadaTipoDfe,
    DataFatoGeradorNaoInformada,
    ErroAvaliadorExpressaoAritmetica,
    ErroFaltaImplementacao,
    ErroGenericoValidacao,
    ErroInternoSistema,
    ErroXml,
    Sistema,
    FormaAplicacaoNaoDefinida,
    FormaAplicacaoNaoTratada,
    FundamentacaoClassificacaoNaoEncontrada,
    HttpMessageNotReadable,
    ImpostoSeletivoInformadoIndevidamente,
    ImpostoSeletivoNaoAdmitidoTipoDfe,
    ImpostoSeletivoNaoInformado,
    IncompatibilidadeSuspensao,
    IncorrectResultSizeDataAccess,
    IndicadorOperacaoNaoEncontrado,
    MethodArgumentTypeMismatch,
    MunicipioNaoEncontrado,
    MunicipioNaoPertencente,
    NbsCompletoNaoInformado,
    NbsNaoEncontrada,
    NbsNaoVinculada,
    NcmCompletoNaoInformado,
    NcmNaoEncontrada,
    NcmNaoVinculada,
    NcmNbsSimultaneas,
    Nomenclatura,
    NomenclaturaIncompativelTipoDfe,
    NomenclaturaNaoPermitidaTipoDfe,
    NomenclaturaObrigatoriaNaoInformada,
    PercentualReducaoNaoEncontrado,
    PlataformaDownloadNaoEncontrada,
    SiglaDFeNaoEncontrada,
    SituacaoTributariaNaoEncontrada,
    TipoAliquotaDesconhecido,
    TipoDfeNaoEncontrado,
    TratamentoClassificacaoNaoEncontrado,
    TratamentoTributarioNaoEncontrado,
    TributacaoRegularInformadaIndevidamente,
    TributacaoRegularNaoInformada,
    TributoSituacaoTributariaNaoEncontrado,
    UfNaoEncontrada,
}

impl Classe {
    pub fn categoria(self) -> Categoria {
        match self {
            Self::AliquotaAdRemNaoEncontrada => Categoria::NaoEncontrada,
            Self::AliquotaImpostoSeletivoNaoInformada => Categoria::Validacao,
            Self::AliquotaNaoEncontrada => Categoria::NaoEncontrada,
            Self::AliquotaNegativa => Categoria::Validacao,
            Self::AliquotaPadraoNaoEncontrada => Categoria::NaoEncontrada,
            Self::AliquotaReferenciaNaoEncontrada => Categoria::NaoEncontrada,
            Self::AliquotasNominaisInformadasIndevidamente => Categoria::Validacao,
            Self::AliquotasNominaisNaoInformadas => Categoria::Validacao,
            Self::BaseCalculoComponentesInconsistente => Categoria::Validacao,
            Self::BaseCalculoInconsistente => Categoria::Validacao,
            Self::BaseCalculoMenorBaseCalculoImpostoSeletivo => Categoria::Validacao,
            Self::BaseCalculoNaoInformada => Categoria::Validacao,
            Self::CampoInvalido => Categoria::BadRequest,
            Self::Captcha => Categoria::BadRequest,
            Self::ClassificacaoTributariaNaoEncontrada => Categoria::NaoEncontrada,
            Self::ClassificacaoTributariaNaoVinculadaSituacaoTributaria => Categoria::Validacao,
            Self::ClassificacaoTributariaNaoVinculadaTipoDfe => Categoria::Validacao,
            Self::DataFatoGeradorNaoInformada => Categoria::BadRequest,
            Self::ErroAvaliadorExpressaoAritmetica => Categoria::Validacao,
            Self::ErroFaltaImplementacao => Categoria::Validacao,
            Self::ErroGenericoValidacao => Categoria::Validacao,
            Self::ErroInternoSistema => Categoria::Interno,
            Self::ErroXml => Categoria::Validacao,
            Self::Sistema => Categoria::Interno,
            Self::FormaAplicacaoNaoDefinida => Categoria::Interno,
            Self::FormaAplicacaoNaoTratada => Categoria::Interno,
            Self::FundamentacaoClassificacaoNaoEncontrada => Categoria::NaoEncontrada,
            Self::HttpMessageNotReadable => Categoria::BadRequest,
            Self::ImpostoSeletivoInformadoIndevidamente => Categoria::Validacao,
            Self::ImpostoSeletivoNaoAdmitidoTipoDfe => Categoria::Validacao,
            Self::ImpostoSeletivoNaoInformado => Categoria::Validacao,
            Self::IncompatibilidadeSuspensao => Categoria::Validacao,
            Self::IncorrectResultSizeDataAccess => Categoria::Interno,
            Self::IndicadorOperacaoNaoEncontrado => Categoria::NaoEncontrada,
            Self::MethodArgumentTypeMismatch => Categoria::BadRequest,
            Self::MunicipioNaoEncontrado => Categoria::NaoEncontrada,
            Self::MunicipioNaoPertencente => Categoria::Validacao,
            Self::NbsCompletoNaoInformado => Categoria::Validacao,
            Self::NbsNaoEncontrada => Categoria::NaoEncontrada,
            Self::NbsNaoVinculada => Categoria::Validacao,
            Self::NcmCompletoNaoInformado => Categoria::Validacao,
            Self::NcmNaoEncontrada => Categoria::NaoEncontrada,
            Self::NcmNaoVinculada => Categoria::Validacao,
            Self::NcmNbsSimultaneas => Categoria::Validacao,
            Self::Nomenclatura => Categoria::Validacao,
            Self::NomenclaturaIncompativelTipoDfe => Categoria::Validacao,
            Self::NomenclaturaNaoPermitidaTipoDfe => Categoria::Validacao,
            Self::NomenclaturaObrigatoriaNaoInformada => Categoria::Validacao,
            Self::PercentualReducaoNaoEncontrado => Categoria::NaoEncontrada,
            Self::PlataformaDownloadNaoEncontrada => Categoria::NaoEncontrada,
            Self::SiglaDFeNaoEncontrada => Categoria::Validacao,
            Self::SituacaoTributariaNaoEncontrada => Categoria::NaoEncontrada,
            Self::TipoAliquotaDesconhecido => Categoria::Validacao,
            Self::TipoDfeNaoEncontrado => Categoria::Validacao,
            Self::TratamentoClassificacaoNaoEncontrado => Categoria::NaoEncontrada,
            Self::TratamentoTributarioNaoEncontrado => Categoria::NaoEncontrada,
            Self::TributacaoRegularInformadaIndevidamente => Categoria::Validacao,
            Self::TributacaoRegularNaoInformada => Categoria::Validacao,
            Self::TributoSituacaoTributariaNaoEncontrado => Categoria::NaoEncontrada,
            Self::UfNaoEncontrada => Categoria::NaoEncontrada,
        }
    }

    pub fn problema(self) -> &'static Problema {
        match self {
            Self::AliquotaAdRemNaoEncontrada => &ALIQUOTA_AD_REM_NAO_ENCONTRADA,
            Self::AliquotaImpostoSeletivoNaoInformada => &ALIQUOTA_IMPOSTO_SELETIVO_NAO_INFORMADA,
            Self::AliquotaNaoEncontrada => &ALIQUOTA_NAO_ENCONTRADA,
            Self::AliquotaNegativa => &ALIQUOTA_NEGATIVA,
            Self::AliquotaPadraoNaoEncontrada => &ALIQUOTA_PADRAO_NAO_ENCONTRADA,
            Self::AliquotaReferenciaNaoEncontrada => &ALIQUOTA_REFERENCIA_NAO_ENCONTRADA,
            Self::AliquotasNominaisInformadasIndevidamente => {
                &ALIQUOTAS_NOMINAIS_INFORMADAS_INDEVIDAMENTE
            }
            Self::AliquotasNominaisNaoInformadas => &ALIQUOTAS_NOMINAIS_NAO_INFORMADAS,
            Self::BaseCalculoComponentesInconsistente => &BASE_CALCULO_COMPONENTES_INCONSISTENTE,
            Self::BaseCalculoInconsistente => &BASE_CALCULO_INCONSISTENTE,
            Self::BaseCalculoMenorBaseCalculoImpostoSeletivo => {
                &BASE_CALCULO_MENOR_BASE_CALCULO_IMPOSTO_SELETIVO
            }
            Self::BaseCalculoNaoInformada => &BASE_CALCULO_NAO_INFORMADA,
            Self::CampoInvalido => &CAMPO_INVALIDO,
            Self::Captcha => &ERRO_CAPTCHA,
            Self::ClassificacaoTributariaNaoEncontrada => &CLASSIFICACAO_TRIBUTARIA_NAO_ENCONTRADA,
            Self::ClassificacaoTributariaNaoVinculadaSituacaoTributaria => {
                &CLASSIFICACAO_TRIBUTARIA_NAO_VINCULADA_SITUACAO_TRIBUTARIA
            }
            Self::ClassificacaoTributariaNaoVinculadaTipoDfe => {
                &CLASSIFICACAO_TRIBUTARIA_NAO_VINCULADA_TIPO_DOCUMENTO
            }
            Self::DataFatoGeradorNaoInformada => &DATA_FATO_GERADOR_NAO_INFORMADA,
            Self::ErroAvaliadorExpressaoAritmetica => &ERRO_AVALIADOR_EXPRESSAO,
            Self::ErroFaltaImplementacao => &ERRO_FALTA_IMPLEMENTACAO,
            Self::ErroGenericoValidacao => &ERRO_GENERICO_VALIDACAO,
            Self::ErroInternoSistema => &ERRO_INTERNO_SISTEMA,
            Self::ErroXml => &ERRO_XML,
            Self::Sistema => &ERRO_SISTEMA,
            Self::FormaAplicacaoNaoDefinida => &FORMA_APLICACAO_NAO_DEFINIDA,
            Self::FormaAplicacaoNaoTratada => &ERRO_SISTEMA,
            Self::FundamentacaoClassificacaoNaoEncontrada => &FUNDAMENTACAO_NAO_ENCONTRADA,
            Self::HttpMessageNotReadable => &ERRO_SISTEMA,
            Self::ImpostoSeletivoInformadoIndevidamente => {
                &IMPOSTO_SELETIVO_INFORMADO_INDEVIDAMENTE
            }
            Self::ImpostoSeletivoNaoAdmitidoTipoDfe => {
                &IMPOSTO_SELETIVO_NAO_ADMITIDO_TIPO_DOCUMENTO
            }
            Self::ImpostoSeletivoNaoInformado => &IMPOSTO_SELETIVO_NAO_INFORMADO,
            Self::IncompatibilidadeSuspensao => &INCOMPATIBILIDADE_SUSPENSAO,
            Self::IncorrectResultSizeDataAccess => &ERRO_SISTEMA,
            Self::IndicadorOperacaoNaoEncontrado => &ERRO_SISTEMA,
            Self::MethodArgumentTypeMismatch => &METHOD_ARGUMENT_TYPE_MISMATCH,
            Self::MunicipioNaoEncontrado => &MUNICIPIO_NAO_ENCONTRADO,
            Self::MunicipioNaoPertencente => &MUNICIPIO_NAO_PERTENCE_UF,
            Self::NbsCompletoNaoInformado => &NBS_COMPLETO_NAO_INFORMADO,
            Self::NbsNaoEncontrada => &NBS_NAO_ENCONTRADO,
            Self::NbsNaoVinculada => &NBS_NAO_VINCULADA,
            Self::NcmCompletoNaoInformado => &NCM_COMPLETO_NAO_INFORMADO,
            Self::NcmNaoEncontrada => &NCM_NAO_ENCONTRADO,
            Self::NcmNaoVinculada => &NCM_NAO_VINCULADA,
            Self::NcmNbsSimultaneas => &NCM_E_NBS_SIMULTANEAS,
            Self::Nomenclatura => &ERRO_NOMENCLATURA,
            Self::NomenclaturaIncompativelTipoDfe => &NOMENCLATURA_INCOMPATIVEL_TIPO_DOCUMENTO,
            Self::NomenclaturaNaoPermitidaTipoDfe => &NOMENCLATURA_NAO_PERMITIDA_TIPO_DOCUMENTO,
            Self::NomenclaturaObrigatoriaNaoInformada => &NOMENCLATURA_OBRIGATORIA_NAO_INFORMADA,
            Self::PercentualReducaoNaoEncontrado => &PERCENTUAL_REDUCAO_NAO_ENCONTRADO,
            Self::PlataformaDownloadNaoEncontrada => &PLATAFORMA_DOWNLOAD_NAO_ENCONTRADA,
            Self::SiglaDFeNaoEncontrada => &SIGLA_DFE_NAO_ENCONTRADA,
            Self::SituacaoTributariaNaoEncontrada => &SITUACAO_TRIBUTARIA_NAO_ENCONTRADA,
            Self::TipoAliquotaDesconhecido => &TIPO_ALIQUOTA_DESCONHECIDO,
            Self::TipoDfeNaoEncontrado => &TIPO_DOCUMENTO_FISCAL_NAO_ENCONTRADO,
            Self::TratamentoClassificacaoNaoEncontrado => &TRATAMENTO_CLASSIFICACAO_NAO_ENCONTRADO,
            Self::TratamentoTributarioNaoEncontrado => &TRATAMENTO_TRIBUTARIO_NAO_ENCONTRADO,
            Self::TributacaoRegularInformadaIndevidamente => {
                &GRUPO_TRIBUTACAO_REGULAR_INFORMADO_INDEVIDAMENTE
            }
            Self::TributacaoRegularNaoInformada => &GRUPO_TRIBUTACAO_REGULAR_NAO_INFORMADO,
            Self::TributoSituacaoTributariaNaoEncontrado => {
                &TRIBUTO_SITUACAO_TRIBUTARIA_NAO_ENCONTRADO
            }
            Self::UfNaoEncontrada => &UF_NAO_ENCONTRADA,
        }
    }
}

pub(super) static CAMPO_INVALIDO: Problema = Problema {
    titulo: "Campo inválido",
    path: "campo-invalido",
    codigo: Some(CodigoErro {
        codigo: "REG-024",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some("O valor informado para o campo não é válido conforme as regras de negócio."),
};
pub(super) static NCM_E_NBS_SIMULTANEAS: Problema = Problema {
    titulo: "NCM e NBS informadas simultaneamente",
    path: "ncm-nbs-simultaneas",
    codigo: Some(CodigoErro {
        codigo: "REG-001",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "Apenas uma nomenclatura (NCM ou NBS) deve ser informada por item. A presença simultânea impede a classificação.",
    ),
};
pub(super) static NCM_NAO_ENCONTRADO: Problema = Problema {
    titulo: "NCM não encontrada",
    path: "ncm-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "REG-002",
        campo: Some("ncm"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O código NCM informado não foi localizado na base de nomenclaturas vigente para a data do fato gerador.",
    ),
};
pub(super) static NBS_NAO_ENCONTRADO: Problema = Problema {
    titulo: "NBS não encontrada",
    path: "nbs-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "REG-003",
        campo: Some("nbs"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O código NBS informado não foi localizado na base de nomenclaturas vigente para a data do fato gerador.",
    ),
};
pub(super) static SITUACAO_TRIBUTARIA_NAO_ENCONTRADA: Problema = Problema {
    titulo: "Situação tributária não encontrada",
    path: "situacao-tributaria-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "REG-004",
        campo: Some("cst"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A situação tributária (CST) informada não existe na base de dados para o tributo e data do fato gerador.",
    ),
};
pub(super) static CLASSIFICACAO_TRIBUTARIA_NAO_ENCONTRADA: Problema = Problema {
    titulo: "Classificação tributária não encontrada",
    path: "classificacao-tributaria-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "REG-005",
        campo: Some("cClassTrib"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A classificação tributária informada não foi localizada na base vigente para a data do fato gerador.",
    ),
};
pub(super) static CLASSIFICACAO_TRIBUTARIA_NAO_VINCULADA_SITUACAO_TRIBUTARIA: Problema = Problema {
    titulo: "Classificação tributária não vinculada à situação tributária",
    path: "classificacao-tributaria-nao-vinculada-situacao-tributaria",
    codigo: Some(CodigoErro {
        codigo: "REG-006",
        campo: Some("cClassTrib"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A classificação tributária informada não está vinculada à situação tributária selecionada.",
    ),
};
pub(super) static ERRO_NOMENCLATURA: Problema = Problema {
    titulo: "Erro de nomenclatura",
    path: "erro-nomenclatura",
    codigo: Some(CodigoErro {
        codigo: "REG-007",
        campo: Some("ncm"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A nomenclatura informada apresenta inconsistência com as regras de classificação tributária vigentes.",
    ),
};
pub(super) static TRATAMENTO_CLASSIFICACAO_NAO_ENCONTRADO: Problema = Problema {
    titulo: "Tratamento de classificação não encontrado",
    path: "tratamento-classificacao-nao-encontrado",
    codigo: Some(CodigoErro {
        codigo: "REG-008",
        campo: Some("cClassTrib"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "Não foi encontrado tratamento de classificação tributária para os parâmetros informados.",
    ),
};
pub(super) static GRUPO_TRIBUTACAO_REGULAR_NAO_INFORMADO: Problema = Problema {
    titulo: "Grupo de tributação regular não informado",
    path: "grupo-tributacao-regular-nao-informado",
    codigo: Some(CodigoErro {
        codigo: "REG-009",
        campo: Some("tributacaoRegular"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O grupo de tributação regular é obrigatório para a situação tributária selecionada e não foi informado.",
    ),
};
pub(super) static INCOMPATIBILIDADE_SUSPENSAO: Problema = Problema {
    titulo: "Incompatibilidade com suspensão",
    path: "incompatibilidade-suspensao",
    codigo: Some(CodigoErro {
        codigo: "REG-010",
        campo: Some("tributacaoRegular/cClassTrib"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A classificação tributária informada é incompatível com a situação de suspensão aplicada ao item.",
    ),
};
pub(super) static NCM_NAO_VINCULADA: Problema = Problema {
    titulo: "NCM não vinculada",
    path: "ncm-nao-vinculada",
    codigo: Some(CodigoErro {
        codigo: "REG-011",
        campo: Some("ncm"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O código NCM informado não está vinculado a nenhuma classificação tributária vigente.",
    ),
};
pub(super) static NBS_NAO_VINCULADA: Problema = Problema {
    titulo: "NBS não vinculada",
    path: "nbs-nao-vinculada",
    codigo: Some(CodigoErro {
        codigo: "REG-012",
        campo: Some("nbs"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O código NBS informado não está vinculado a nenhuma classificação tributária vigente.",
    ),
};
pub(super) static IMPOSTO_SELETIVO_NAO_INFORMADO: Problema = Problema {
    titulo: "Dados do Imposto Seletivo não informados",
    path: "dados-imposto-seletivo-nao-informados",
    codigo: Some(CodigoErro {
        codigo: "REG-013",
        campo: Some("impostoSeletivo"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "Os dados do Imposto Seletivo são obrigatórios para a classificação tributária selecionada e não foram informados.",
    ),
};
pub(super) static IMPOSTO_SELETIVO_INFORMADO_INDEVIDAMENTE: Problema = Problema {
    titulo: "Dados do Imposto Seletivo informados indevidamente",
    path: "dados-imposto-seletivo-informados-indevidamente",
    codigo: Some(CodigoErro {
        codigo: "REG-014",
        campo: Some("impostoSeletivo"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "Os dados do Imposto Seletivo foram informados, mas a classificação tributária selecionada não prevê incidência.",
    ),
};
pub(super) static NCM_COMPLETO_NAO_INFORMADO: Problema = Problema {
    titulo: "NCM completo não informado",
    path: "ncm-completo-nao-informado",
    codigo: Some(CodigoErro {
        codigo: "REG-015",
        campo: Some("ncm"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O código NCM deve ser informado com todos os dígitos (completo) para a operação solicitada.",
    ),
};
pub(super) static NBS_COMPLETO_NAO_INFORMADO: Problema = Problema {
    titulo: "NBS completo não informado",
    path: "nbs-completo-nao-informado",
    codigo: Some(CodigoErro {
        codigo: "REG-016",
        campo: Some("nbs"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O código NBS deve ser informado com todos os dígitos (completo) para a operação solicitada.",
    ),
};
pub(super) static MUNICIPIO_NAO_PERTENCE_UF: Problema = Problema {
    titulo: "Município não pertencente à UF",
    path: "municipio-nao-pertencente-uf",
    codigo: Some(CodigoErro {
        codigo: "REG-018",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some("O município informado não pertence à UF indicada na operação."),
};
pub(super) static MUNICIPIO_NAO_ENCONTRADO: Problema = Problema {
    titulo: "Município não encontrado",
    path: "municipio-nao-encontrado",
    codigo: Some(CodigoErro {
        codigo: "REG-019",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some("O código do município informado não foi localizado na base de dados."),
};
pub(super) static UF_NAO_ENCONTRADA: Problema = Problema {
    titulo: "UF não encontrada",
    path: "uf-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "REG-020",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some("A UF informada para a operação não foi encontrada na base de dados."),
};
pub(super) static DATA_FATO_GERADOR_NAO_INFORMADA: Problema = Problema {
    titulo: "Data do fato gerador não informada",
    path: "fato-gerador-nao-informada",
    codigo: Some(CodigoErro {
        codigo: "REG-021",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some("A data do fato gerador é obrigatória e não foi informada na requisição."),
};
pub(super) static GRUPO_TRIBUTACAO_REGULAR_INFORMADO_INDEVIDAMENTE: Problema = Problema {
    titulo: "Grupo de tributação regular informado indevidamente",
    path: "grupo-tributacao-regular-informado-indevidamente",
    codigo: Some(CodigoErro {
        codigo: "REG-022",
        campo: Some("tributacaoRegular"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O grupo de tributação regular foi informado, mas a situação tributária selecionada não o exige.",
    ),
};
pub(super) static ERRO_GENERICO_VALIDACAO: Problema = Problema {
    titulo: "Erro genérico de validação",
    path: "erro-generico-validacao",
    codigo: Some(CodigoErro {
        codigo: "REG-023",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some("Erro de validação que não se enquadra em nenhuma categoria específica."),
};
pub(super) static BASE_CALCULO_NAO_INFORMADA: Problema = Problema {
    titulo: "Base de cálculo não informada",
    path: "base-calculo-nao-informada",
    codigo: Some(CodigoErro {
        codigo: "REG-025",
        campo: Some("baseCalculo"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A base de cálculo de CBS e IBS é obrigatória quando o grupo do Imposto Seletivo não é informado.",
    ),
};
pub(super) static BASE_CALCULO_INCONSISTENTE: Problema = Problema {
    titulo: "Base de cálculo inconsistente",
    path: "base-calculo-inconsistente",
    codigo: Some(CodigoErro {
        codigo: "REG-026",
        campo: Some("baseCalculo"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A base de cálculo de CBS e IBS informada difere da soma da base de cálculo do Imposto Seletivo com o Imposto Seletivo informado.",
    ),
};
pub(super) static BASE_CALCULO_MENOR_BASE_CALCULO_IMPOSTO_SELETIVO: Problema = Problema {
    titulo: "Base de cálculo menor que a base de cálculo do Imposto Seletivo",
    path: "base-calculo-menor-base-calculo-imposto-seletivo",
    codigo: Some(CodigoErro {
        codigo: "REG-027",
        campo: Some("baseCalculo"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A base de cálculo de CBS e IBS informada é menor que a base de cálculo do Imposto Seletivo.",
    ),
};
pub(super) static TIPO_DOCUMENTO_FISCAL_NAO_ENCONTRADO: Problema = Problema {
    titulo: "Tipo de documento fiscal não encontrado",
    path: "tipo-documento-fiscal-nao-encontrado",
    codigo: Some(CodigoErro {
        codigo: "REG-028",
        campo: Some("tpDoc"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O tipo de documento fiscal (tpDoc) informado na operação não existe ou está fora de vigência na data do fato gerador.",
    ),
};
pub(super) static CLASSIFICACAO_TRIBUTARIA_NAO_VINCULADA_TIPO_DOCUMENTO: Problema = Problema {
    titulo: "Classificação tributária não vinculada ao tipo de documento",
    path: "classificacao-tributaria-nao-vinculada-tipo-documento",
    codigo: Some(CodigoErro {
        codigo: "REG-029",
        campo: Some("cClassTrib"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A classificação tributária informada não possui vínculo vigente com o tipo de documento fiscal (tpDoc) da operação.",
    ),
};
pub(super) static NOMENCLATURA_INCOMPATIVEL_TIPO_DOCUMENTO: Problema = Problema {
    titulo: "Nomenclatura incompatível com o tipo de documento",
    path: "nomenclatura-incompativel-tipo-documento",
    codigo: Some(CodigoErro {
        codigo: "REG-030",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A nomenclatura informada (NCM ou NBS) não é aceita para a classificação tributária no tipo de documento fiscal (tpDoc) da operação.",
    ),
};
pub(super) static NOMENCLATURA_NAO_PERMITIDA_TIPO_DOCUMENTO: Problema = Problema {
    titulo: "Nomenclatura não permitida para o tipo de documento",
    path: "nomenclatura-nao-permitida-tipo-documento",
    codigo: Some(CodigoErro {
        codigo: "REG-031",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O tipo de documento fiscal (tpDoc) da operação não admite o preenchimento de NCM ou NBS.",
    ),
};
pub(super) static IMPOSTO_SELETIVO_NAO_ADMITIDO_TIPO_DOCUMENTO: Problema = Problema {
    titulo: "Imposto Seletivo não admitido pelo tipo de documento",
    path: "imposto-seletivo-nao-admitido-tipo-documento",
    codigo: Some(CodigoErro {
        codigo: "REG-032",
        campo: Some("impostoSeletivo"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "O NCM/NBS informado é tributado pelo Imposto Seletivo, mas o tipo de documento fiscal (tpDoc) da operação não admite Imposto Seletivo.",
    ),
};
pub(super) static NOMENCLATURA_OBRIGATORIA_NAO_INFORMADA: Problema = Problema {
    titulo: "Nomenclatura obrigatória não informada",
    path: "nomenclatura-obrigatoria-nao-informada",
    codigo: Some(CodigoErro {
        codigo: "REG-033",
        campo: None,
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A classificação tributária possui anexo com NCM e NBS aplicáveis e exige o preenchimento de um deles, completo, para o tipo de documento fiscal (tpDoc) da operação.",
    ),
};
pub(super) static BASE_CALCULO_COMPONENTES_INCONSISTENTE: Problema = Problema {
    titulo: "Base de cálculo inconsistente",
    path: "base-calculo-inconsistente",
    codigo: Some(CodigoErro {
        codigo: "REG-034",
        campo: Some("baseCalculo"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A base de cálculo declarada no item diverge da base calculada a partir dos componentes informados",
    ),
};
pub(super) static ALIQUOTA_IMPOSTO_SELETIVO_NAO_INFORMADA: Problema = Problema {
    titulo: "Alíquota do Imposto Seletivo não informada",
    path: "aliquota-imposto-seletivo-nao-informada",
    codigo: Some(CodigoErro {
        codigo: "REG-035",
        campo: Some("aliquotasNominais/impostoSeletivo"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "A NCM/NBS possui incidência do Imposto Seletivo, mas a alíquota ainda não foi definida em lei e não foi informada pelo usuário.",
    ),
};
pub(super) static ALIQUOTAS_NOMINAIS_INFORMADAS_INDEVIDAMENTE: Problema = Problema {
    titulo: "Alíquotas nominais informadas indevidamente",
    path: "aliquotas-nominais-informadas-indevidamente",
    codigo: Some(CodigoErro {
        codigo: "REG-036",
        campo: Some("aliquotasNominais"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "As alíquotas nominais não podem ser informadas para fato gerador anterior a 01/01/2027, pois a calculadora resolve as alíquotas vigentes.",
    ),
};
pub(super) static ALIQUOTAS_NOMINAIS_NAO_INFORMADAS: Problema = Problema {
    titulo: "Alíquotas nominais não informadas",
    path: "aliquotas-nominais-nao-informadas",
    codigo: Some(CodigoErro {
        codigo: "REG-037",
        campo: Some("aliquotasNominais"),
        estado: EstadoItem::InconsistenciaEntrada,
    }),
    descricao: Some(
        "As alíquotas nominais (cbs, ibsEstadual e ibsMunicipal) devem ser informadas para fato gerador a partir de 01/01/2027.",
    ),
};
pub(super) static ALIQUOTA_NAO_ENCONTRADA: Problema = Problema {
    titulo: "Alíquota não encontrada",
    path: "aliquota-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "CAL-001",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "Não foi possível localizar a alíquota aplicável para os parâmetros tributários informados.",
    ),
};
pub(super) static ALIQUOTA_PADRAO_NAO_ENCONTRADA: Problema = Problema {
    titulo: "Alíquota padrão não encontrada",
    path: "aliquota-padrao-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "CAL-002",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "A alíquota padrão (de referência) não foi encontrada na base para o tributo e período informados.",
    ),
};
pub(super) static ALIQUOTA_REFERENCIA_NAO_ENCONTRADA: Problema = Problema {
    titulo: "Alíquota de referência não encontrada",
    path: "aliquota-referencia-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "CAL-003",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "A alíquota de referência necessária para o cálculo não está disponível na base de dados.",
    ),
};
pub(super) static ALIQUOTA_AD_REM_NAO_ENCONTRADA: Problema = Problema {
    titulo: "Alíquota ad rem não encontrada",
    path: "aliquota-ad-rem-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "CAL-004",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "A alíquota ad rem (valor fixo por unidade) não foi localizada para o produto e período informados.",
    ),
};
pub(super) static ALIQUOTA_NEGATIVA: Problema = Problema {
    titulo: "Alíquota negativa",
    path: "aliquota-negativa",
    codigo: Some(CodigoErro {
        codigo: "CAL-005",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "O cálculo resultou em uma alíquota negativa, o que indica inconsistência nos parâmetros tributários.",
    ),
};
pub(super) static TIPO_ALIQUOTA_DESCONHECIDO: Problema = Problema {
    titulo: "Tipo de alíquota desconhecido",
    path: "tipo-aliquota-desconhecido",
    codigo: Some(CodigoErro {
        codigo: "CAL-006",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "O tipo de alíquota retornado pela base de dados não é reconhecido pelo sistema.",
    ),
};
pub(super) static ERRO_FALTA_IMPLEMENTACAO: Problema = Problema {
    titulo: "Classificação tributária em desenvolvimento",
    path: "erro-falta-implementacao",
    codigo: Some(CodigoErro {
        codigo: "CAL-007",
        campo: Some("cClassTrib"),
        estado: EstadoItem::NaoImplementado,
    }),
    descricao: Some(
        "A classificação tributária informada ainda não possui implementação completa no sistema.",
    ),
};
pub(super) static ERRO_AVALIADOR_EXPRESSAO: Problema = Problema {
    titulo: "Erro no avaliador de expressão aritmética",
    path: "erro-avaliador-expressao",
    codigo: Some(CodigoErro {
        codigo: "CAL-008",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "Ocorreu um erro ao avaliar a expressão aritmética utilizada no cálculo do tributo.",
    ),
};
pub(super) static TRATAMENTO_TRIBUTARIO_NAO_ENCONTRADO: Problema = Problema {
    titulo: "Tratamento tributário não encontrado",
    path: "tratamento-tributario-nao-encontrado",
    codigo: Some(CodigoErro {
        codigo: "CAL-009",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "O tratamento tributário aplicável não foi localizado para a combinação de parâmetros informada.",
    ),
};
pub(super) static PERCENTUAL_REDUCAO_NAO_ENCONTRADO: Problema = Problema {
    titulo: "Percentual de redução não encontrado",
    path: "percentual-reducao-nao-encontrado",
    codigo: Some(CodigoErro {
        codigo: "CAL-010",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "O percentual de redução de alíquota previsto para a classificação não foi encontrado na base.",
    ),
};
pub(super) static FUNDAMENTACAO_NAO_ENCONTRADA: Problema = Problema {
    titulo: "Fundamentação não encontrada",
    path: "fundamentacao-nao-encontrada",
    codigo: Some(CodigoErro {
        codigo: "CAL-011",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "A fundamentação legal para a classificação tributária não foi localizada na base de dados.",
    ),
};
pub(super) static TRIBUTO_SITUACAO_TRIBUTARIA_NAO_ENCONTRADO: Problema = Problema {
    titulo: "Tributo/situação tributária não encontrado",
    path: "tributo-situacao-tributaria-nao-encontrado",
    codigo: Some(CodigoErro {
        codigo: "CAL-012",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "A combinação de tributo e situação tributária informada não foi localizada na base.",
    ),
};
pub(super) static ERRO_INTERNO_SISTEMA: Problema = Problema {
    titulo: "Erro interno do sistema",
    path: "erro-interno-de-sistema",
    codigo: Some(CodigoErro {
        codigo: "CAL-999",
        campo: None,
        estado: EstadoItem::FalhaTecnica,
    }),
    descricao: Some(
        "Erro interno inesperado durante o processamento do cálculo. Contate o suporte se o problema persistir.",
    ),
};
pub(super) static FORMA_APLICACAO_NAO_DEFINIDA: Problema = Problema {
    titulo: "Forma de Aplicação de Percentual não definida",
    path: "forma-aplicacao-percentual-nao-definida",
    codigo: None,
    descricao: None,
};
pub(super) static ERRO_XML: Problema = Problema {
    titulo: "Erro de validação de XML",
    path: "erro-xml",
    codigo: None,
    descricao: None,
};
pub(super) static ERRO_NEGOCIO: Problema = Problema {
    titulo: "Violação de regra de negócio",
    path: "erro-negocio",
    codigo: None,
    descricao: None,
};
pub(super) static ERRO_CAPTCHA: Problema = Problema {
    titulo: "Erro de Captcha",
    path: "erro-captcha",
    codigo: None,
    descricao: None,
};
pub(super) static METHOD_ARGUMENT_TYPE_MISMATCH: Problema = Problema {
    titulo: "Tipo de argumento inválido",
    path: "tipo-argumento-invalido",
    codigo: None,
    descricao: None,
};
pub(super) static SIGLA_DFE_NAO_ENCONTRADA: Problema = Problema {
    titulo: "Sigla DFe não reconhecida",
    path: "sigla-dfe-nao-reconhecida",
    codigo: None,
    descricao: None,
};
pub(super) static PLATAFORMA_DOWNLOAD_NAO_ENCONTRADA: Problema = Problema {
    titulo: "Plataforma de download não encontrada",
    path: "plataforma-download-nao-encontrada",
    codigo: None,
    descricao: None,
};
pub(super) static ERRO_SISTEMA: Problema = Problema {
    titulo: "Erro de sistema não previsto",
    path: "erro-de-sistema-nao-previsto",
    codigo: None,
    descricao: None,
};

pub(super) static TODOS: &[&Problema] = &[
    &CAMPO_INVALIDO,
    &NCM_E_NBS_SIMULTANEAS,
    &NCM_NAO_ENCONTRADO,
    &NBS_NAO_ENCONTRADO,
    &SITUACAO_TRIBUTARIA_NAO_ENCONTRADA,
    &CLASSIFICACAO_TRIBUTARIA_NAO_ENCONTRADA,
    &CLASSIFICACAO_TRIBUTARIA_NAO_VINCULADA_SITUACAO_TRIBUTARIA,
    &ERRO_NOMENCLATURA,
    &TRATAMENTO_CLASSIFICACAO_NAO_ENCONTRADO,
    &GRUPO_TRIBUTACAO_REGULAR_NAO_INFORMADO,
    &INCOMPATIBILIDADE_SUSPENSAO,
    &NCM_NAO_VINCULADA,
    &NBS_NAO_VINCULADA,
    &IMPOSTO_SELETIVO_NAO_INFORMADO,
    &IMPOSTO_SELETIVO_INFORMADO_INDEVIDAMENTE,
    &NCM_COMPLETO_NAO_INFORMADO,
    &NBS_COMPLETO_NAO_INFORMADO,
    &MUNICIPIO_NAO_PERTENCE_UF,
    &MUNICIPIO_NAO_ENCONTRADO,
    &UF_NAO_ENCONTRADA,
    &DATA_FATO_GERADOR_NAO_INFORMADA,
    &GRUPO_TRIBUTACAO_REGULAR_INFORMADO_INDEVIDAMENTE,
    &ERRO_GENERICO_VALIDACAO,
    &BASE_CALCULO_NAO_INFORMADA,
    &BASE_CALCULO_INCONSISTENTE,
    &BASE_CALCULO_MENOR_BASE_CALCULO_IMPOSTO_SELETIVO,
    &TIPO_DOCUMENTO_FISCAL_NAO_ENCONTRADO,
    &CLASSIFICACAO_TRIBUTARIA_NAO_VINCULADA_TIPO_DOCUMENTO,
    &NOMENCLATURA_INCOMPATIVEL_TIPO_DOCUMENTO,
    &NOMENCLATURA_NAO_PERMITIDA_TIPO_DOCUMENTO,
    &IMPOSTO_SELETIVO_NAO_ADMITIDO_TIPO_DOCUMENTO,
    &NOMENCLATURA_OBRIGATORIA_NAO_INFORMADA,
    &BASE_CALCULO_COMPONENTES_INCONSISTENTE,
    &ALIQUOTA_IMPOSTO_SELETIVO_NAO_INFORMADA,
    &ALIQUOTAS_NOMINAIS_INFORMADAS_INDEVIDAMENTE,
    &ALIQUOTAS_NOMINAIS_NAO_INFORMADAS,
    &ALIQUOTA_NAO_ENCONTRADA,
    &ALIQUOTA_PADRAO_NAO_ENCONTRADA,
    &ALIQUOTA_REFERENCIA_NAO_ENCONTRADA,
    &ALIQUOTA_AD_REM_NAO_ENCONTRADA,
    &ALIQUOTA_NEGATIVA,
    &TIPO_ALIQUOTA_DESCONHECIDO,
    &ERRO_FALTA_IMPLEMENTACAO,
    &ERRO_AVALIADOR_EXPRESSAO,
    &TRATAMENTO_TRIBUTARIO_NAO_ENCONTRADO,
    &PERCENTUAL_REDUCAO_NAO_ENCONTRADO,
    &FUNDAMENTACAO_NAO_ENCONTRADA,
    &TRIBUTO_SITUACAO_TRIBUTARIA_NAO_ENCONTRADO,
    &ERRO_INTERNO_SISTEMA,
    &FORMA_APLICACAO_NAO_DEFINIDA,
    &ERRO_XML,
    &ERRO_NEGOCIO,
    &ERRO_CAPTCHA,
    &METHOD_ARGUMENT_TYPE_MISMATCH,
    &SIGLA_DFE_NAO_ENCONTRADA,
    &PLATAFORMA_DOWNLOAD_NAO_ENCONTRADA,
    &ERRO_SISTEMA,
];
