//! validação de NCM/NBS contra os anexos da classificação e o tipo de DFe

use chrono::NaiveDate;

use super::Servicos;
use super::texto::{data_java, preenchido, tamanho};
use crate::db::{ClassificacaoTributaria, Nomenclatura, TipoDfe};
use crate::erro::{Classe, Falha, Resultado};

pub const TAM_NCM_COMPLETO: usize = 8;
pub const TAM_NBS_COMPLETO: usize = 9;
const CLASSIFICACAO_EXCECAO: &str = "200038";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiglaDfe {
    NfeAbi,
    Nfe,
    Nfce,
    Cte,
    CteOs,
    CteSimplificado,
    Bpe,
    BpeTa,
    BpeTm,
    Nf3e,
    Nfse,
    NfseVia,
    Nfcom,
    Dere,
    Dir,
    Duimp,
    Nfag,
    Nfgas,
}

impl SiglaDfe {
    const TODAS: [(Self, &'static str, &'static str); 18] = [
        (Self::NfeAbi, "NFEABI", "NFe ABI"),
        (Self::Nfe, "NFE", "NFe"),
        (Self::Nfce, "NFCE", "NFCe"),
        (Self::Cte, "CTE", "CTe"),
        (Self::CteOs, "CTEOS", "CTe OS"),
        (
            Self::CteSimplificado,
            "CTESIMPLIFICADO",
            "CT-e Simplificado",
        ),
        (Self::Bpe, "BPE", "BPe"),
        (Self::BpeTa, "BPETA", "BPe TA"),
        (Self::BpeTm, "BPETM", "BPe TM"),
        (Self::Nf3e, "NF3E", "NF3e"),
        (Self::Nfse, "NFSE", "NFSe"),
        (Self::NfseVia, "NFSEVIA", "NFSe Via"),
        (Self::Nfcom, "NFCOM", "NFCom"),
        (Self::Dere, "DERE", "DERE"),
        (Self::Dir, "DIR", "DIR"),
        (Self::Duimp, "DUIMP", "Duimp"),
        (Self::Nfag, "NFAG", "NFAg"),
        (Self::Nfgas, "NFGAS", "NFGas"),
    ];

    /// `normalizarSigla`: maiúsculas sem `[\s\-_]`
    pub fn normalizar(sigla: &str) -> String {
        sigla
            .to_uppercase()
            .chars()
            .filter(|c| {
                !matches!(
                    c,
                    ' ' | '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r' | '-' | '_'
                )
            })
            .collect()
    }

    fn por_chave(chave: &str) -> Option<Self> {
        Self::TODAS
            .iter()
            .find(|(_, k, _)| *k == chave)
            .map(|(s, _, _)| *s)
    }

    pub fn por_sigla(sigla: &str) -> Resultado<Self> {
        Self::por_chave(&Self::normalizar(sigla)).ok_or_else(|| {
            Falha::new(
                Classe::SiglaDFeNaoEncontrada,
                format!("Sigla DFe inválida: {sigla}"),
            )
        })
    }

    pub fn chave(self) -> &'static str {
        Self::TODAS
            .iter()
            .find(|(s, _, _)| *s == self)
            .map_or("", |(_, k, _)| k)
    }

    pub fn sigla(self) -> &'static str {
        Self::TODAS
            .iter()
            .find(|(s, _, _)| *s == self)
            .map_or("", |(_, _, sigla)| sigla)
    }

    pub fn corresponde(self, sigla: &str) -> bool {
        self.chave() == Self::normalizar(sigla)
    }

    pub fn sem_tratamento_calculadora(self) -> bool {
        matches!(self, Self::NfeAbi | Self::Dere)
    }

    pub fn is_aplicavel_por_sigla(sigla: &str) -> bool {
        matches!(Self::normalizar(sigla).as_str(), "NFE" | "NFCE")
    }

    fn ncm_aplicavel(self) -> bool {
        matches!(self, Self::Nfse | Self::Nfe | Self::Nfce)
    }

    fn nbs_aplicavel(self) -> bool {
        self == Self::Nfse
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultadoNomenclatura {
    Ncm,
    ExcecaoNcm,
    Nbs,
    ExcecaoNbs,
    Misto,
    Sem,
}

impl ResultadoNomenclatura {
    fn nome(self) -> &'static str {
        match self {
            Self::Ncm => "NCM",
            Self::ExcecaoNcm => "EXCECAO_NCM",
            Self::Nbs => "NBS",
            Self::ExcecaoNbs => "EXCECAO_NBS",
            Self::Misto => "MISTO",
            Self::Sem => "SEM",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modo {
    Permissivo,
    Rigido,
}

/// classificação alvo da validação (id para as consultas, código para as mensagens)
#[derive(Debug, Clone, Copy)]
pub struct Alvo<'a> {
    pub id: i64,
    pub codigo: &'a str,
    pub tributos: &'a str,
}

fn ncm_completo_nao_informado() -> Falha {
    Falha::new(
        Classe::NcmCompletoNaoInformado,
        "NCM completo não informado",
    )
}

fn nbs_completo_nao_informado() -> Falha {
    Falha::new(
        Classe::NbsCompletoNaoInformado,
        "NBS completo não informado",
    )
}

fn incompativel(nomenclatura: &str, codigo: &str, sigla: SiglaDfe) -> Falha {
    Falha::new(
        Classe::NomenclaturaIncompativelTipoDfe,
        format!(
            "{nomenclatura} não permitida para a Classificação Tributária de código {codigo} no tipo de documento {}",
            sigla.sigla()
        ),
    )
}

pub(crate) fn nao_encontrada(n: Nomenclatura, codigo: &str, dia: NaiveDate) -> Falha {
    let (classe, nome) = match n {
        Nomenclatura::Ncm => (Classe::NcmNaoEncontrada, "NCM"),
        Nomenclatura::Nbs => (Classe::NbsNaoEncontrada, "NBS"),
    };
    Falha::new(
        classe,
        format!(
            "{nome} de código {codigo} não encontrada para a data {}",
            data_java(dia)
        ),
    )
}

fn nao_vinculada(n: Nomenclatura, codigo_nom: &str, alvo: Alvo<'_>) -> Falha {
    let (classe, nome) = match n {
        Nomenclatura::Ncm => (Classe::NcmNaoVinculada, "NCM"),
        Nomenclatura::Nbs => (Classe::NbsNaoVinculada, "NBS"),
    };
    Falha::new(
        classe,
        format!(
            "{nome} de código {codigo_nom} não vinculada à Classificação Tributária de código {} ({})",
            alvo.codigo, alvo.tributos
        ),
    )
}

impl Servicos<'_> {
    pub fn validar_aplicavel(
        &self,
        n: Nomenclatura,
        codigo: Option<&str>,
        alvo: Alvo<'_>,
        dia: NaiveDate,
    ) -> Resultado<()> {
        let (completo, falha): (usize, fn() -> Falha) = match n {
            Nomenclatura::Ncm => (TAM_NCM_COMPLETO, ncm_completo_nao_informado),
            Nomenclatura::Nbs => (TAM_NBS_COMPLETO, nbs_completo_nao_informado),
        };
        let tem = self.db.classificacao_tem_aplicavel(n, alvo.id, dia)?;
        let informado_completo = tamanho(codigo) == completo;
        if tem && !informado_completo {
            return Err(falha());
        }
        match codigo {
            Some(c) if informado_completo => self.criticar(n, c, alvo, dia, tem),
            _ => Ok(()),
        }
    }

    fn criticar_contra_anexo(
        &self,
        n: Nomenclatura,
        codigo: &str,
        alvo: Alvo<'_>,
        dia: NaiveDate,
    ) -> Resultado<()> {
        let tem = self.db.classificacao_tem_aplicavel(n, alvo.id, dia)?;
        self.criticar(n, codigo, alvo, dia, tem)
    }

    fn criticar(
        &self,
        n: Nomenclatura,
        codigo: &str,
        alvo: Alvo<'_>,
        dia: NaiveDate,
        tem_anexo: bool,
    ) -> Resultado<()> {
        if !tem_anexo {
            return Ok(());
        }
        if !self.db.tem_aplicavel(n, codigo, alvo.id, dia)? {
            return Err(nao_vinculada(n, codigo, alvo));
        }
        if self.db.tem_excecao_aplicavel(n, codigo, alvo.id, dia)?
            && !self.db.tem_aplicavel_sem_excecao(n, codigo, alvo.id, dia)?
        {
            return Err(nao_vinculada(n, codigo, alvo));
        }
        Ok(())
    }

    pub fn validar_vinculo_tipo_dfe(
        &self,
        tipo_dfe: &TipoDfe,
        alvo: Alvo<'_>,
        dia: NaiveDate,
    ) -> Resultado<()> {
        let vinculado = self
            .db
            .tipos_dfe_da_classificacao(alvo.id, dia)?
            .iter()
            .any(|t| t.id == tipo_dfe.id);
        if vinculado {
            return Ok(());
        }
        let tipo = tipo_dfe
            .tipo
            .map_or_else(|| "null".to_string(), |t| t.to_string());
        Err(Falha::new(
            Classe::ClassificacaoTributariaNaoVinculadaTipoDfe,
            format!(
                "Classificação Tributária de código {} não vinculada ao tipo de documento fiscal (tpDoc) {tipo} ({})",
                alvo.codigo, alvo.tributos
            ),
        ))
    }

    fn determinar_nomenclatura(
        &self,
        sigla: SiglaDfe,
        codigo: &str,
        dia: NaiveDate,
    ) -> Resultado<ResultadoNomenclatura> {
        let classificacao = self.classificacao_cbs_ibs(Some(codigo), dia)?;
        self.nomenclatura_da_classificacao(sigla, &classificacao, dia)
    }

    /// versão do endpoint, que recebe a sigla crua: a classificação é buscada
    /// antes da sigla ser validada
    pub fn determinar_nomenclatura_por_sigla(
        &self,
        sigla: &str,
        codigo: &str,
        dia: NaiveDate,
    ) -> Resultado<(SiglaDfe, &'static str)> {
        let classificacao = self.classificacao_cbs_ibs(Some(codigo), dia)?;
        let sigla = SiglaDfe::por_sigla(sigla)?;
        let resultado = self.nomenclatura_da_classificacao(sigla, &classificacao, dia)?;
        Ok((sigla, resultado.nome()))
    }

    fn nomenclatura_da_classificacao(
        &self,
        sigla: SiglaDfe,
        classificacao: &ClassificacaoTributaria,
        dia: NaiveDate,
    ) -> Resultado<ResultadoNomenclatura> {
        let compativel = self
            .db
            .tipos_dfe_da_classificacao(classificacao.id, dia)?
            .iter()
            .any(|t| sigla.corresponde(&t.sigla));
        if !compativel {
            return Ok(ResultadoNomenclatura::Sem);
        }
        let ncm = self
            .db
            .classificacao_tem_aplicavel(Nomenclatura::Ncm, classificacao.id, dia)?;
        let nbs = self
            .db
            .classificacao_tem_aplicavel(Nomenclatura::Nbs, classificacao.id, dia)?;
        Ok(resultado_nomenclatura(
            sigla,
            &classificacao.codigo,
            ncm,
            nbs,
        ))
    }

    pub fn validar_nomenclatura(
        &self,
        sigla: SiglaDfe,
        alvo: Alvo<'_>,
        ncm: Option<&str>,
        nbs: Option<&str>,
        dia: NaiveDate,
        modo: Modo,
    ) -> Resultado<()> {
        match self.determinar_nomenclatura(sigla, alvo.codigo, dia)? {
            ResultadoNomenclatura::Ncm | ResultadoNomenclatura::ExcecaoNcm => {
                self.exigir(Nomenclatura::Ncm, sigla, alvo, ncm, nbs, dia)
            }
            ResultadoNomenclatura::Nbs | ResultadoNomenclatura::ExcecaoNbs => {
                self.exigir(Nomenclatura::Nbs, sigla, alvo, nbs, ncm, dia)
            }
            ResultadoNomenclatura::Misto => self.exigir_ncm_ou_nbs(sigla, alvo, ncm, nbs, dia),
            ResultadoNomenclatura::Sem => {
                self.validar_sem_exigencia(sigla, alvo, ncm, nbs, dia, modo)
            }
        }
    }

    /// `exigirNcm`/`exigirNbs`: `outra` é a nomenclatura que não pode vir junto
    fn exigir(
        &self,
        n: Nomenclatura,
        sigla: SiglaDfe,
        alvo: Alvo<'_>,
        codigo: Option<&str>,
        outra: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<()> {
        let (nome_outra, completo, falha): (&str, usize, fn() -> Falha) = match n {
            Nomenclatura::Ncm => ("NBS", TAM_NCM_COMPLETO, ncm_completo_nao_informado),
            Nomenclatura::Nbs => ("NCM", TAM_NBS_COMPLETO, nbs_completo_nao_informado),
        };
        if preenchido(outra) {
            return Err(incompativel(nome_outra, alvo.codigo, sigla));
        }
        match codigo {
            Some(c) if tamanho(codigo) == completo => self.criticar_contra_anexo(n, c, alvo, dia),
            _ => Err(falha()),
        }
    }

    fn exigir_ncm_ou_nbs(
        &self,
        sigla: SiglaDfe,
        alvo: Alvo<'_>,
        ncm: Option<&str>,
        nbs: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<()> {
        let completo = |n: Nomenclatura, c: Option<&str>, tam: usize, falha: fn() -> Falha| match c
        {
            Some(c) if tamanho(Some(c)) == tam => self.criticar_contra_anexo(n, c, alvo, dia),
            _ => Err(falha()),
        };
        if preenchido(ncm) {
            completo(
                Nomenclatura::Ncm,
                ncm,
                TAM_NCM_COMPLETO,
                ncm_completo_nao_informado,
            )
        } else if preenchido(nbs) {
            completo(
                Nomenclatura::Nbs,
                nbs,
                TAM_NBS_COMPLETO,
                nbs_completo_nao_informado,
            )
        } else {
            Err(Falha::new(
                Classe::NomenclaturaObrigatoriaNaoInformada,
                format!(
                    "A Classificação Tributária de código {} exige NCM ou NBS completo para o tipo de documento {}",
                    alvo.codigo,
                    sigla.sigla()
                ),
            ))
        }
    }

    fn validar_sem_exigencia(
        &self,
        sigla: SiglaDfe,
        alvo: Alvo<'_>,
        ncm: Option<&str>,
        nbs: Option<&str>,
        dia: NaiveDate,
        modo: Modo,
    ) -> Resultado<()> {
        let ncm_informado = preenchido(ncm);
        let nbs_informado = preenchido(nbs);
        if !ncm_informado && !nbs_informado {
            return Ok(());
        }
        if matches!(sigla, SiglaDfe::Nfe | SiglaDfe::Nfce | SiglaDfe::Nfse) {
            if sigla == SiglaDfe::Nfse {
                if ncm_informado {
                    return Err(incompativel("NCM", alvo.codigo, sigla));
                }
            } else if nbs_informado {
                return Err(incompativel("NBS", alvo.codigo, sigla));
            }
            return Ok(());
        }
        if modo == Modo::Rigido {
            return Err(Falha::new(
                Classe::NomenclaturaNaoPermitidaTipoDfe,
                format!(
                    "NCM/NBS não permitido para a Classificação Tributária de código {} no tipo de documento {}",
                    alvo.codigo,
                    sigla.sigla()
                ),
            ));
        }
        if ncm_informado {
            self.validar_aplicavel(Nomenclatura::Ncm, ncm, alvo, dia)
        } else {
            self.validar_aplicavel(Nomenclatura::Nbs, nbs, alvo, dia)
        }
    }
}

fn resultado_nomenclatura(
    sigla: SiglaDfe,
    codigo: &str,
    ncm: bool,
    nbs: bool,
) -> ResultadoNomenclatura {
    if codigo == CLASSIFICACAO_EXCECAO {
        return match sigla {
            SiglaDfe::Nfe | SiglaDfe::Nfce => ResultadoNomenclatura::ExcecaoNcm,
            SiglaDfe::Nfse => ResultadoNomenclatura::ExcecaoNbs,
            _ => ResultadoNomenclatura::Sem,
        };
    }
    if sigla.ncm_aplicavel() && !sigla.nbs_aplicavel() {
        return if ncm {
            ResultadoNomenclatura::Ncm
        } else {
            ResultadoNomenclatura::Sem
        };
    }
    if sigla.nbs_aplicavel() {
        return match (ncm, nbs) {
            (true, true) => ResultadoNomenclatura::Misto,
            (true, false) => ResultadoNomenclatura::Ncm,
            (false, true) => ResultadoNomenclatura::Nbs,
            (false, false) => ResultadoNomenclatura::Sem,
        };
    }
    ResultadoNomenclatura::Sem
}
