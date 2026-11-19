//! `ObservabilidadeService`: cálculo item a item com erros detalhados por item

use super::Servicos;
use super::calculadora::Contexto;
use crate::erro::{EstadoItem, Falha, Problema};
use crate::json::{ErroEscrita, Escrever, Escritor};
use crate::modelo::entrada::{Item, Operacao};
use crate::modelo::roc::{OperacaoConsumo, Tributos, ValoresTotais};
use crate::modelo::validacao::{Violacao, validar_item, validar_operacao};
use crate::saida_json;

const CODIGO_FALHA_TECNICA: &str = "CAL-999";
const TITULO_VALIDACAO_PADRAO: &str = "Erro de validação";
const INDICE_OPERACAO: i64 = -1;

/// `CodigoValidacao`: (anotação, código, título, descrição)
const CODIGOS_VALIDACAO: [(&str, &str, &str, &str); 16] = [
    (
        "NotNull",
        "VAL-001",
        "Campo obrigatório não informado",
        "O campo marcado como obrigatório não foi preenchido na requisição.",
    ),
    (
        "NotBlank",
        "VAL-002",
        "Campo obrigatório em branco",
        "O campo não pode estar em branco ou conter apenas espaços.",
    ),
    (
        "NotEmpty",
        "VAL-003",
        "Lista ou campo não pode estar vazio",
        "O campo ou lista informado não pode estar vazio.",
    ),
    (
        "Size",
        "VAL-004",
        "Tamanho fora do intervalo permitido",
        "O valor informado não atende aos limites de tamanho mínimo e/ou máximo definidos.",
    ),
    (
        "Min",
        "VAL-005",
        "Valor abaixo do mínimo permitido",
        "O valor informado é inferior ao mínimo aceito para o campo.",
    ),
    (
        "Max",
        "VAL-006",
        "Valor acima do máximo permitido",
        "O valor informado é superior ao máximo aceito para o campo.",
    ),
    (
        "DecimalMin",
        "VAL-007",
        "Valor decimal abaixo do mínimo permitido",
        "O valor decimal informado é inferior ao mínimo aceito para o campo.",
    ),
    (
        "DecimalMax",
        "VAL-008",
        "Valor decimal acima do máximo permitido",
        "O valor decimal informado é superior ao máximo aceito para o campo.",
    ),
    (
        "Pattern",
        "VAL-009",
        "Formato inválido",
        "O valor informado não atende ao padrão (formato) exigido para o campo.",
    ),
    (
        "Digits",
        "VAL-010",
        "Quantidade de dígitos inválida",
        "O valor informado não possui a quantidade de dígitos inteiros e/ou decimais esperada.",
    ),
    (
        "Email",
        "VAL-011",
        "E-mail inválido",
        "O endereço de e-mail informado não é válido.",
    ),
    (
        "Positive",
        "VAL-012",
        "Valor deve ser positivo",
        "O valor informado deve ser um número positivo (maior que zero).",
    ),
    (
        "PositiveOrZero",
        "VAL-013",
        "Valor deve ser positivo ou zero",
        "O valor informado deve ser um número positivo ou zero.",
    ),
    (
        "AssertTrue",
        "VAL-014",
        "Condição não atendida",
        "A condição de validação (assert true) não foi atendida pelo valor informado.",
    ),
    (
        "AssertFalse",
        "VAL-015",
        "Condição atendida indevidamente",
        "A condição de validação (assert false) foi atendida quando não deveria.",
    ),
    (
        "_Desconhecido",
        "VAL-000",
        "Erro de validação não categorizado",
        "O tipo de validação aplicado não possui uma categoria específica.",
    ),
];
const CODIGO_DESCONHECIDO: &str = "VAL-000";

fn codigo_validacao(anotacao: &str) -> &'static str {
    CODIGOS_VALIDACAO
        .iter()
        .find(|(a, ..)| *a == anotacao)
        .map_or(CODIGO_DESCONHECIDO, |(_, c, ..)| c)
}

fn titulo_validacao(codigo: &str) -> &'static str {
    CODIGOS_VALIDACAO
        .iter()
        .find(|(_, c, ..)| *c == codigo)
        .map_or(TITULO_VALIDACAO_PADRAO, |(_, _, t, _)| t)
}

/// `ErroDescricao` do `GET /calculadora/observabilidade/erros`
#[derive(Debug, Clone)]
pub struct ErroDescricao {
    pub codigo: Option<String>,
    pub titulo: Option<String>,
    pub descricao: Option<String>,
}

saida_json!(ErroDescricao {
    "codigo" => codigo: texto,
    "titulo" => titulo: texto,
    "descricao" => descricao: texto,
});

pub fn descrever_erro(codigo: &str) -> Option<ErroDescricao> {
    if let Some(p) = Problema::por_codigo(codigo) {
        return Some(ErroDescricao {
            codigo: Some(codigo.to_string()),
            titulo: Some(p.titulo.to_string()),
            descricao: p.descricao.map(str::to_string),
        });
    }
    CODIGOS_VALIDACAO
        .iter()
        .find(|(_, c, ..)| *c == codigo)
        .map(|(_, c, t, d)| ErroDescricao {
            codigo: Some((*c).to_string()),
            titulo: Some((*t).to_string()),
            descricao: Some((*d).to_string()),
        })
}

#[derive(Debug, Clone)]
pub struct ErroDetalhe {
    pub tipo: Option<String>,
    pub titulo: Option<String>,
    pub codigo: Option<String>,
    pub ponteiro: Option<String>,
    pub detalhe: Option<String>,
}

saida_json!(ErroDetalhe {
    "type" => tipo: texto,
    "title" => titulo: texto,
    "code" => codigo: texto,
    "pointer" => ponteiro: texto,
    "detail" => detalhe: texto,
});

#[derive(Debug, Clone)]
pub struct EstadoSaida(pub EstadoItem);

impl Escrever for EstadoSaida {
    fn escrever(&self, e: &mut Escritor) -> Result<(), ErroEscrita> {
        e.valor_string(self.0.nome());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ItemObservabilidade {
    pub n_obj: Option<i64>,
    pub estado_item: Option<EstadoSaida>,
    pub trib_calc: Option<Tributos>,
    pub erros: Option<Vec<ErroDetalhe>>,
}

saida_json!(ItemObservabilidade {
    "nObj" => n_obj: inteiro,
    "estadoItem" => estado_item: objeto,
    "tribCalc" => trib_calc: objeto,
    "erros" => erros: lista,
});

#[derive(Debug, Clone, Default)]
pub struct RocObservabilidade {
    pub oper: Option<OperacaoConsumo>,
    pub objetos: Option<Vec<ItemObservabilidade>>,
    pub total: Option<ValoresTotais>,
}

saida_json!(RocObservabilidade {
    "oper" => oper: objeto,
    "objetos" => objetos: lista,
    "total" => total: objeto,
});

/// resultado de erro: 422 com a lista (`ErrosCalculoException`) ou falha não tratada
#[derive(Debug)]
pub enum ErroObservabilidade {
    Erros(Vec<ErroDetalhe>),
    /// falha ao preparar a operação (UF, município, data, tpDoc); o Java loga antes do 422
    Operacao {
        erros: Vec<ErroDetalhe>,
        causa: Falha,
    },
    Falha(Falha),
}

impl From<Falha> for ErroObservabilidade {
    fn from(f: Falha) -> Self {
        Self::Falha(f)
    }
}

fn detalhe_validacao(campo: &str, v: &Violacao) -> String {
    match &v.valor {
        Some(valor) => format!("Campo '{campo}' {}, recebido: '{valor}'", v.mensagem),
        None => format!("Campo '{campo}' {}", v.mensagem),
    }
}

/// `toJsonPointer`: `[n]` vira `/n` e `.` vira `/`
fn ponteiro_json(campo: &str) -> String {
    let mut s = String::with_capacity(campo.len() + 2);
    s.push_str("#/");
    let mut resto = campo;
    while let Some(abre) = resto.find('[') {
        let depois = &resto[abre + 1..];
        let fim_digitos = depois
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(depois.len());
        if fim_digitos > 0 && depois[fim_digitos..].starts_with(']') {
            s.push_str(&resto[..abre]);
            s.push('/');
            s.push_str(&depois[..fim_digitos]);
            resto = &depois[fim_digitos + 1..];
        } else {
            s.push_str(&resto[..=abre]);
            resto = depois;
        }
    }
    s.push_str(resto);
    s.replace('.', "/")
}

struct Coletor<'a> {
    indice: i64,
    base_url: &'a str,
    erros: Vec<ErroDetalhe>,
    estado: EstadoItem,
}

impl<'a> Coletor<'a> {
    fn new(indice: i64, base_url: &'a str) -> Self {
        Self {
            indice,
            base_url,
            erros: Vec::new(),
            estado: EstadoItem::Calculado,
        }
    }

    fn tipo(&self, codigo: &str) -> String {
        format!(
            "{}/api/calculadora/observabilidade/erros?codigo={codigo}",
            self.base_url
        )
    }

    fn ponteiro(&self, campo: Option<&str>) -> String {
        match campo {
            Some(c) => format!("#/itens/{}/{c}", self.indice),
            None => format!("#/itens/{}", self.indice),
        }
    }

    fn validacao(&mut self, v: &Violacao) {
        if self.estado == EstadoItem::Calculado {
            self.estado = EstadoItem::InconsistenciaEntrada;
        }
        let codigo = codigo_validacao(v.restricao.nome());
        self.erros.push(ErroDetalhe {
            tipo: Some(self.tipo(codigo)),
            titulo: Some(titulo_validacao(codigo).to_string()),
            codigo: Some(codigo.to_string()),
            ponteiro: Some(self.ponteiro(Some(&v.caminho))),
            detalhe: Some(detalhe_validacao(&v.caminho, v)),
        });
    }

    fn falha(&mut self, f: &Falha) {
        let p = f.problema();
        let (codigo, campo) = match &p.codigo {
            Some(c) => {
                self.estado = c.estado;
                (c.codigo, c.campo)
            }
            None => {
                self.estado = EstadoItem::FalhaTecnica;
                (CODIGO_FALHA_TECNICA, None)
            }
        };
        self.erros.push(ErroDetalhe {
            tipo: Some(self.tipo(codigo)),
            titulo: Some(p.titulo.to_string()),
            codigo: Some(codigo.to_string()),
            ponteiro: Some(self.ponteiro(campo)),
            detalhe: f.mensagem.clone(),
        });
    }
}

impl Servicos<'_> {
    pub fn calcular_observabilidade(
        &self,
        op: &Operacao,
        base_url: &str,
    ) -> Result<RocObservabilidade, ErroObservabilidade> {
        let globais = erros_globais(op, base_url);
        if !globais.is_empty() {
            return Err(ErroObservabilidade::Erros(globais));
        }
        let ctx = self.contexto(op).map_err(|f| {
            let mut c = Coletor::new(INDICE_OPERACAO, base_url);
            c.falha(&f);
            ErroObservabilidade::Operacao {
                erros: c.erros,
                causa: f,
            }
        })?;
        let mut resultados = Vec::new();
        for (i, item) in op.itens.iter().flatten().enumerate() {
            let item = item.as_ref().ok_or_else(|| {
                Falha::sistema("java.lang.IllegalArgumentException: HV000116: The object to be validated must not be null.")
            })?;
            resultados.push(self.item_observabilidade(op, &ctx, item, i, base_url));
        }
        resultados.sort_by_key(|r| r.n_obj);
        let calculados: Vec<&Tributos> = resultados
            .iter()
            .filter(|r| {
                r.estado_item.as_ref().is_some_and(|e| {
                    matches!(e.0, EstadoItem::Calculado | EstadoItem::CalculoSimulado)
                })
            })
            .filter_map(|r| r.trib_calc.as_ref())
            .collect();
        let total = (!calculados.is_empty()).then(|| ValoresTotais::de(&calculados));
        Ok(RocObservabilidade {
            oper: self.operacao_consumo(op, ctx.dia)?,
            objetos: Some(resultados),
            total,
        })
    }

    fn item_observabilidade(
        &self,
        op: &Operacao,
        ctx: &Contexto,
        item: &Item,
        indice: usize,
        base_url: &str,
    ) -> ItemObservabilidade {
        let indice = i64::try_from(indice).unwrap_or(i64::MAX);
        let n_obj = Some(item.numero.map_or(indice + 1, i64::from));
        let mut c = Coletor::new(indice, base_url);
        for v in validar_item(item) {
            c.validacao(&v);
        }
        if c.erros.is_empty() {
            match self.processar_item_no_contexto(op, ctx, item) {
                Ok(tributos) => {
                    let estado = if item.aliquotas_nominais.is_none() {
                        EstadoItem::Calculado
                    } else {
                        EstadoItem::CalculoSimulado
                    };
                    return ItemObservabilidade {
                        n_obj,
                        estado_item: Some(EstadoSaida(estado)),
                        trib_calc: Some(tributos),
                        erros: None,
                    };
                }
                Err(f) => c.falha(&f),
            }
        }
        ItemObservabilidade {
            n_obj,
            estado_item: Some(EstadoSaida(c.estado)),
            trib_calc: None,
            erros: Some(c.erros),
        }
    }
}

/// `validarCamposGlobais`: violações fora de `itens`
fn erros_globais(op: &Operacao, base_url: &str) -> Vec<ErroDetalhe> {
    validar_operacao(op)
        .into_iter()
        .filter(|v| !v.caminho.starts_with("itens[") && !v.caminho.starts_with("itens."))
        .map(|v| {
            let codigo = codigo_validacao(v.restricao.nome());
            ErroDetalhe {
                tipo: Some(format!(
                    "{base_url}/calculadora/observabilidade/erros?codigo={codigo}"
                )),
                titulo: Some(titulo_validacao(codigo).to_string()),
                codigo: Some(codigo.to_string()),
                ponteiro: Some(ponteiro_json(&v.caminho)),
                detalhe: Some(detalhe_validacao(&v.caminho, &v)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ponteiro_como_regex_java() {
        assert_eq!(
            ponteiro_json("gCompraGov.tpEnteGov"),
            "#/gCompraGov/tpEnteGov"
        );
        assert_eq!(ponteiro_json("itens[10].cst"), "#/itens/10/cst");
        assert_eq!(ponteiro_json("itens"), "#/itens");
    }

    #[test]
    fn codigos() {
        assert_eq!(codigo_validacao("NotNull"), "VAL-001");
        assert_eq!(codigo_validacao("Xyz"), "VAL-000");
        assert!(descrever_erro("VAL-013").is_some());
        assert!(descrever_erro("NADA").is_none());
    }
}
