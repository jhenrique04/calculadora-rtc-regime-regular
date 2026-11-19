//! saída do `Marshaller` do JAXB com `JAXB_FORMATTED_OUTPUT`: recuo de quatro
//! espaços, grupo sem filhos como `<a/>` e texto vazio como `<a></a>`

const CABECALHO: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";
const RECUO: &str = "    ";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Conteudo {
    Texto(String),
    Filhos(Vec<Elemento>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Elemento {
    nome: &'static str,
    atributo: Option<(&'static str, String)>,
    conteudo: Conteudo,
}

impl Elemento {
    pub fn texto(nome: &'static str, valor: String) -> Self {
        Self {
            nome,
            atributo: None,
            conteudo: Conteudo::Texto(valor),
        }
    }

    pub fn grupo(nome: &'static str, filhos: Vec<Elemento>) -> Self {
        Self {
            nome,
            atributo: None,
            conteudo: Conteudo::Filhos(filhos),
        }
    }

    pub fn com_atributo(mut self, nome: &'static str, valor: String) -> Self {
        self.atributo = Some((nome, valor));
        self
    }
}

pub fn documento(raiz: &Elemento, namespace: &str) -> String {
    let mut saida = String::from(CABECALHO);
    escrever(&mut saida, raiz, 0, Some(namespace));
    saida
}

fn escrever(saida: &mut String, e: &Elemento, nivel: usize, namespace: Option<&str>) {
    saida.push_str(&RECUO.repeat(nivel));
    saida.push('<');
    saida.push_str(e.nome);
    if let Some(ns) = namespace {
        saida.push_str(" xmlns=\"");
        escapar(saida, ns, true);
        saida.push('"');
    }
    if let Some((nome, valor)) = &e.atributo {
        saida.push(' ');
        saida.push_str(nome);
        saida.push_str("=\"");
        escapar(saida, valor, true);
        saida.push('"');
    }
    match &e.conteudo {
        Conteudo::Filhos(filhos) if filhos.is_empty() => saida.push_str("/>\n"),
        Conteudo::Filhos(filhos) => {
            saida.push_str(">\n");
            for f in filhos {
                escrever(saida, f, nivel + 1, None);
            }
            saida.push_str(&RECUO.repeat(nivel));
            fechar(saida, e.nome);
        }
        Conteudo::Texto(t) => {
            saida.push('>');
            escapar(saida, t, false);
            fechar(saida, e.nome);
        }
    }
}

fn fechar(saida: &mut String, nome: &str) {
    saida.push_str("</");
    saida.push_str(nome);
    saida.push_str(">\n");
}

/// `MinimumEscapeHandler`: só `&`, `<`, `>`, aspas em atributo e CR
fn escapar(saida: &mut String, texto: &str, atributo: bool) {
    for c in texto.chars() {
        match c {
            '&' => saida.push_str("&amp;"),
            '<' => saida.push_str("&lt;"),
            '>' => saida.push_str("&gt;"),
            '"' if atributo => saida.push_str("&quot;"),
            '\r' => saida.push_str("&#13;"),
            _ => saida.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formato_do_marshaller() {
        let raiz = Elemento::grupo(
            "raiz",
            vec![
                Elemento::grupo("vazio", vec![]),
                Elemento::grupo("det", vec![Elemento::texto("a", "x<&>\"\r".into())])
                    .com_atributo("nItem", "1".into()),
                Elemento::texto("b", String::new()),
            ],
        );
        assert_eq!(
            documento(&raiz, "urn:x"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
             <raiz xmlns=\"urn:x\">\n    <vazio/>\n    <det nItem=\"1\">\n        \
             <a>x&lt;&amp;&gt;\"&#13;</a>\n    </det>\n    <b></b>\n</raiz>\n"
        );
    }
}
