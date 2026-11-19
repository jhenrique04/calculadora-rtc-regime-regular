//! validação por eventos como o `XMLSchemaValidator` do Xerces: o primeiro erro
//! interrompe a leitura, na mesma ordem de verificações e com as mesmas mensagens

use std::collections::HashSet;

use super::esquema::dfa::{Dfa, EstadoCm, Termo};
use super::esquema::simples::TipoSimples;
use super::esquema::{
    ANY_TYPE, Conteudo, Curinga, Esquema, IdElemento, IdTipo, NS_XSD, Processamento, Tipo,
    TipoComplexo, Unico,
};
use super::leitor::{Atributo, Elemento, ErroXml, Manipulador, NS_XMLNS, Namespaces, Posicao};

const NS_XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
const ATRIBUTOS_XSI: [&str; 4] = ["type", "nil", "schemaLocation", "noNamespaceSchemaLocation"];

type R<T = ()> = Result<T, String>;

struct Quadro<'e> {
    nome: String,
    decl: Option<IdElemento>,
    tipo: IdTipo,
    modelo: Option<(&'e Dfa, EstadoCm)>,
    texto: String,
    viu_filho: bool,
    viu_caracteres: bool,
    viu_texto: bool,
    nil: bool,
    /// subárvore de curinga `skip`: só conta a profundidade
    pulados: Option<usize>,
}

struct EscopoUnico<'e> {
    unico: &'e Unico,
    elemento: &'e str,
    profundidade: usize,
    valores: HashSet<String>,
}

pub struct Validador<'e> {
    esquema: &'e Esquema,
    pilha: Vec<Quadro<'e>>,
    ids: HashSet<String>,
    unicos: Vec<EscopoUnico<'e>>,
}

impl<'e> Validador<'e> {
    pub fn novo(esquema: &'e Esquema) -> Self {
        Self {
            esquema,
            pilha: Vec::new(),
            ids: HashSet::new(),
            unicos: Vec::new(),
        }
    }

    fn descricao_termo(&self, t: &Termo) -> String {
        match t {
            Termo::Elemento(id) => self.esquema.elemento(*id).descricao(),
            Termo::Curinga(c) => c.descricao(),
        }
    }

    fn esperados(&self, dfa: &Dfa, estado: &EstadoCm) -> (usize, String) {
        let lista = dfa.esperados(estado);
        let nomes: Vec<String> = lista
            .iter()
            .map(|t| self.descricao_termo(&dfa.termos[*t]))
            .collect();
        (lista.len(), format!("{{{}}}", nomes.join(", ")))
    }

    fn casa(&self, termo: &Termo, e: &Elemento) -> bool {
        match termo {
            Termo::Elemento(id) => {
                let d = self.esquema.elemento(*id);
                d.nome == e.nome.local && d.ns.as_deref() == e.nome.uri.as_deref()
            }
            Termo::Curinga(c) => c.aceita(e.nome.uri.as_deref()),
        }
    }

    /// erros de `handleStartElement` quando o filho não cabe no modelo do pai
    fn conteudo_invalido(&self, dfa: &Dfa, estado: &EstadoCm, e: &Elemento) -> String {
        let bruto = &e.nome.bruto;
        let (quantos, esperado) = self.esperados(dfa, estado);
        let ocorrencia = dfa.ocorrencia(estado);
        if quantos == 0 {
            return match ocorrencia {
                Some(o) if o.max.is_some_and(|m| estado.contador >= m) => format!(
                    "cvc-complex-type.2.4.f: '{}' can occur a maximum of '{}' times in the current sequence. This limit was exceeded. No child element is expected at this point.",
                    self.descricao_termo(&dfa.termos[o.termo]),
                    o.max.unwrap_or_default()
                ),
                _ => format!(
                    "cvc-complex-type.2.4.d: Invalid content was found starting with element '{bruto}'. No child element is expected at this point."
                ),
            };
        }
        let expandido = match &e.nome.uri {
            Some(u) => format!("{{\"{u}\":{}}}", e.nome.local),
            None => e.nome.local.clone(),
        };
        match ocorrencia {
            Some(o) if estado.contador < o.min => {
                self.minimo_nao_atingido(bruto, &dfa.termos[o.termo], o.min, estado.contador)
            }
            Some(o) if o.max.is_some_and(|m| estado.contador >= m) => format!(
                "cvc-complex-type.2.4.e: '{bruto}' can occur a maximum of '{}' times in the current sequence. This limit was exceeded. At this point one of '{esperado}' is expected.",
                o.max.unwrap_or_default()
            ),
            _ => format!(
                "cvc-complex-type.2.4.a: Invalid content was found starting with element '{expandido}'. One of '{esperado}' is expected."
            ),
        }
    }

    fn minimo_nao_atingido(
        &self,
        elemento: &str,
        termo: &Termo,
        min: u32,
        contador: u32,
    ) -> String {
        let nome = self.descricao_termo(termo);
        let faltam = min - contador;
        if faltam > 1 {
            format!(
                "cvc-complex-type.2.4.j: The content of element '{elemento}' is not complete. '{nome}' expected to appear a minimum of '{min}' times. '{faltam}' more instances are required to satisfy this constraint."
            )
        } else {
            format!(
                "cvc-complex-type.2.4.i: The content of element '{elemento}' is not complete. '{nome}' expected to appear a minimum of '{min}' times. One more instance is required to satisfy this constraint."
            )
        }
    }

    fn simples(&self, id: IdTipo) -> Option<&'e TipoSimples> {
        match self.esquema.tipo(id) {
            Tipo::Simples(s) => Some(s),
            Tipo::Complexo(_) => None,
        }
    }

    /// passo 1: o filho no modelo de conteúdo do pai
    fn no_modelo_do_pai(&mut self, e: &Elemento) -> R<(Option<IdElemento>, Option<&'e Curinga>)> {
        let Some(pai) = self.pilha.last_mut() else {
            return Ok((None, None));
        };
        pai.viu_filho = true;
        let Some((dfa, mut estado)) = pai.modelo else {
            return Ok((None, None));
        };
        let casou = dfa.transicao(&mut estado, |t| self.casa(t, e));
        if let Some(pai) = self.pilha.last_mut() {
            pai.modelo = Some((dfa, estado));
        }
        match casou.map(|t| &dfa.termos[t]) {
            Some(Termo::Elemento(id)) => Ok((Some(*id), None)),
            Some(Termo::Curinga(c)) => Ok((None, Some(c.as_ref()))),
            None => Err(self.conteudo_invalido(dfa, &estado, e)),
        }
    }

    fn tipo_xsi(&self, e: &Elemento, ns: &Namespaces, atual: Option<IdTipo>) -> R<Option<IdTipo>> {
        let Some(a) = atributo_xsi(e, "type") else {
            return Ok(None);
        };
        let valor = a.valor.trim();
        let (prefixo, local) = valor.split_once(':').unwrap_or(("", valor));
        let uri = ns.uri(prefixo);
        if !prefixo.is_empty() && uri.is_none() {
            return Err(format!(
                "UndeclaredPrefix: Cannot resolve '{valor}' as a QName: the prefix '{prefixo}' is not declared."
            ));
        }
        let Some(tipo) = self.esquema.tipo_global(uri, local) else {
            return Err(format!(
                "cvc-elt.4.2: Cannot resolve '{valor}' to a type definition for element '{}'.",
                e.nome.bruto
            ));
        };
        if let Some(atual) = atual
            && !self.esquema.deriva_de(tipo, atual)
        {
            return Err(format!(
                "cvc-elt.4.3: Type '{valor}' is not validly derived from the type definition, '{}', of element '{}'.",
                self.esquema.tipo(atual).nome(),
                e.nome.bruto
            ));
        }
        Ok(Some(tipo))
    }

    fn validar_valor(&mut self, tipo: &TipoSimples, exibido: &str, normalizado: &str) -> R {
        tipo.validar(exibido, normalizado, &mut self.ids)
    }

    fn atributos_simples(&mut self, e: &Elemento) -> R {
        for a in &e.atributos {
            if a.nome.uri.as_deref() == Some(NS_XSI)
                && ATRIBUTOS_XSI.contains(&a.nome.local.as_str())
            {
                self.valor_xsi(a)?;
                continue;
            }
            if a.nome.uri.as_deref() == Some(NS_XMLNS) {
                continue;
            }
            return Err(format!(
                "cvc-type.3.1.1: Element '{}' is a simple type, so it cannot have attributes, excepting those whose namespace name is identical to '{NS_XSI}' and whose [local name] is one of 'type', 'nil', 'schemaLocation' or 'noNamespaceSchemaLocation'. However, the attribute, '{}' was found.",
                e.nome.bruto, a.nome.bruto
            ));
        }
        Ok(())
    }

    fn valor_xsi(&mut self, a: &Atributo) -> R {
        if a.nome.local != "nil" {
            return Ok(());
        }
        let Some(boolean) = self
            .esquema
            .tipo_global(Some(NS_XSD), "boolean")
            .and_then(|t| self.simples(t))
        else {
            return Ok(());
        };
        let normalizado = boolean.espacos.normalizar(&a.valor);
        self.validar_valor(boolean, &a.valor, &normalizado)
    }

    fn atributos_complexos(&mut self, e: &Elemento, tipo: &'e TipoComplexo) -> R {
        for a in &e.atributos {
            let uri = a.nome.uri.as_deref();
            if uri == Some(NS_XSI) && ATRIBUTOS_XSI.contains(&a.nome.local.as_str()) {
                self.valor_xsi(a)?;
                continue;
            }
            if uri == Some(NS_XMLNS) {
                continue;
            }
            let uso = tipo
                .atributos
                .iter()
                .find(|u| u.nome == a.nome.local && u.ns.as_deref() == uri);
            let Some(uso) = uso else {
                if tipo
                    .curinga_atributos
                    .as_ref()
                    .is_some_and(|c| c.aceita(uri) && c.processamento != Processamento::Estrito)
                {
                    continue;
                }
                return Err(format!(
                    "cvc-complex-type.3.2.2: Attribute '{}' is not allowed to appear in element '{}'.",
                    a.nome.bruto, e.nome.bruto
                ));
            };
            let Some(st) = self.simples(uso.tipo) else {
                continue;
            };
            let normalizado = st.espacos.normalizar(&a.valor);
            self.validar_valor(st, &a.valor, &normalizado)?;
            if let Some(fixo) = &uso.fixo
                && st.valor_real(&normalizado) != st.valor_real(&st.espacos.normalizar(fixo))
            {
                return Err(format!(
                    "cvc-complex-type.3.1: Value '{}' of attribute '{}' of element '{}' is not valid with respect to the corresponding attribute use. Attribute '{}' has a fixed value of '{fixo}'.",
                    a.valor, a.nome.bruto, e.nome.bruto, a.nome.bruto
                ));
            }
        }
        for uso in tipo.atributos.iter().filter(|u| u.obrigatorio) {
            let presente = e
                .atributos
                .iter()
                .any(|a| a.nome.local == uso.nome && a.nome.uri.as_deref() == uso.ns.as_deref());
            if !presente {
                return Err(format!(
                    "cvc-complex-type.4: Attribute '{}' must appear on element '{}'.",
                    uso.nome, e.nome.bruto
                ));
            }
        }
        Ok(())
    }

    /// campos `@atributo` das restrições `xs:unique` ativas que selecionam este elemento
    fn identidade(&mut self, e: &Elemento, tipo: IdTipo) -> R {
        let profundidade = self.pilha.len();
        let complexo = match self.esquema.tipo(tipo) {
            Tipo::Complexo(c) => Some(c),
            Tipo::Simples(_) => None,
        };
        for escopo in &mut self.unicos {
            if profundidade != escopo.profundidade + escopo.unico.seletor.profundidade() {
                continue;
            }
            let Some(a) = e
                .atributos
                .iter()
                .find(|a| a.nome.local == escopo.unico.atributo && a.nome.uri.is_none())
            else {
                continue;
            };
            let tipo_atributo = complexo
                .and_then(|c| {
                    c.atributos
                        .iter()
                        .find(|u| u.nome == a.nome.local && u.ns.is_none())
                })
                .and_then(|u| match self.esquema.tipo(u.tipo) {
                    Tipo::Simples(s) => Some(s),
                    Tipo::Complexo(_) => None,
                });
            let valor = match tipo_atributo {
                Some(s) => s.valor_real(&s.espacos.normalizar(&a.valor)),
                None => a.valor.clone(),
            };
            if !escopo.valores.insert(valor.clone()) {
                return Err(format!(
                    "cvc-identity-constraint.4.1: Duplicate unique value [{valor}] declared for identity constraint \"{}\" of element \"{}\".",
                    escopo.unico.nome, escopo.elemento
                ));
            }
        }
        Ok(())
    }

    fn iniciar(&mut self, e: &Elemento, ns: &Namespaces) -> R {
        if let Some(Quadro {
            pulados: Some(n), ..
        }) = self.pilha.last_mut()
        {
            *n += 1;
            return Ok(());
        }
        let (decl, curinga) = self.no_modelo_do_pai(e)?;
        if curinga.is_some_and(|c| c.processamento == Processamento::Pular) {
            self.pilha.push(self.quadro(e, None, ANY_TYPE, Some(0)));
            return Ok(());
        }
        let decl = decl.or_else(|| {
            self.esquema
                .elemento_global(e.nome.uri.as_deref(), &e.nome.local)
        });
        let declarado = decl.map(|d| self.esquema.elemento(d).tipo);
        let xsi = self.tipo_xsi(e, ns, declarado)?;
        let tipo = match (xsi, declarado) {
            (Some(t), _) | (None, Some(t)) => t,
            (None, None) => {
                if self.pilha.is_empty() {
                    return Err(format!(
                        "cvc-elt.1.a: Cannot find the declaration of element '{}'.",
                        e.nome.bruto
                    ));
                }
                if curinga.is_some_and(|c| c.processamento == Processamento::Estrito) {
                    return Err(format!(
                        "cvc-complex-type.2.4.c: The matching wildcard is strict, but no declaration can be found for element '{}'.",
                        e.nome.bruto
                    ));
                }
                ANY_TYPE
            }
        };
        let nil = self.nil(e, decl)?;
        match self.esquema.tipo(tipo) {
            Tipo::Simples(_) => self.atributos_simples(e)?,
            Tipo::Complexo(c) => self.atributos_complexos(e, c)?,
        }
        self.identidade(e, tipo)?;
        if let Some(d) = decl {
            let declaracao = self.esquema.elemento(d);
            for unico in &declaracao.unicos {
                self.unicos.push(EscopoUnico {
                    unico,
                    elemento: &declaracao.nome,
                    profundidade: self.pilha.len(),
                    valores: HashSet::new(),
                });
            }
        }
        let mut q = self.quadro(e, decl, tipo, None);
        q.nil = nil;
        self.pilha.push(q);
        Ok(())
    }

    fn nil(&mut self, e: &Elemento, decl: Option<IdElemento>) -> R<bool> {
        let (Some(a), Some(d)) = (atributo_xsi(e, "nil"), decl) else {
            return Ok(false);
        };
        let declaracao = self.esquema.elemento(d);
        if !declaracao.nillable {
            return Err(format!(
                "cvc-elt.3.1: Attribute '{NS_XSI},nil' must not appear on element '{}', because the {{nillable}} property of '{}' is false.",
                e.nome.bruto, e.nome.bruto
            ));
        }
        self.valor_xsi(a)?;
        Ok(matches!(a.valor.trim(), "true" | "1"))
    }

    fn quadro(
        &self,
        e: &Elemento,
        decl: Option<IdElemento>,
        tipo: IdTipo,
        pulados: Option<usize>,
    ) -> Quadro<'e> {
        let modelo = match self.esquema.tipo(tipo) {
            Tipo::Complexo(c) if pulados.is_none() => c.modelo().map(|d| (d, d.inicio())),
            _ => None,
        };
        Quadro {
            nome: e.nome.bruto.clone(),
            decl,
            tipo,
            modelo,
            texto: String::new(),
            viu_filho: false,
            viu_caracteres: false,
            viu_texto: false,
            nil: false,
            pulados,
        }
    }

    fn finalizar(&mut self) -> R {
        if let Some(Quadro {
            pulados: Some(n), ..
        }) = self.pilha.last_mut()
            && *n > 0
        {
            *n -= 1;
            return Ok(());
        }
        let Some(q) = self.pilha.pop() else {
            return Ok(());
        };
        let profundidade = self.pilha.len();
        self.unicos.retain(|u| u.profundidade != profundidade);
        if q.pulados.is_some() {
            return Ok(());
        }
        match self.esquema.tipo(q.tipo) {
            Tipo::Simples(s) => self.fim_simples(&q, s),
            Tipo::Complexo(c) => self.fim_complexo(&q, c),
        }
    }

    fn conteudo_texto(&self, q: &Quadro<'_>) -> String {
        let padrao = q.decl.and_then(|d| {
            let decl = self.esquema.elemento(d);
            decl.fixo.clone().or_else(|| decl.padrao.clone())
        });
        match padrao {
            Some(p) if !q.viu_texto && !q.viu_filho && !q.nil => p,
            _ => q.texto.clone(),
        }
    }

    fn fim_simples(&mut self, q: &Quadro<'_>, tipo: &TipoSimples) -> R {
        if q.viu_filho {
            return Err(format!(
                "cvc-type.3.1.2: Element '{}' is a simple type, so it must have no element information item [children].",
                q.nome
            ));
        }
        if q.nil {
            return Ok(());
        }
        let normalizado = tipo.espacos.normalizar(&self.conteudo_texto(q));
        self.validar_valor(tipo, &normalizado, &normalizado)?;
        self.verificar_fixo(q, tipo, &normalizado)
    }

    fn verificar_fixo(&self, q: &Quadro<'_>, tipo: &TipoSimples, normalizado: &str) -> R {
        let Some(fixo) = q.decl.and_then(|d| self.esquema.elemento(d).fixo.as_ref()) else {
            return Ok(());
        };
        if tipo.valor_real(normalizado) == tipo.valor_real(&tipo.espacos.normalizar(fixo)) {
            return Ok(());
        }
        Err(format!(
            "cvc-elt.5.2.2.2.2: The value '{normalizado}' of element '{}' does not match the {{value constraint}} value '{fixo}'.",
            q.nome
        ))
    }

    fn fim_complexo(&mut self, q: &Quadro<'_>, tipo: &TipoComplexo) -> R {
        if q.nil {
            return Ok(());
        }
        match &tipo.conteudo {
            Conteudo::Vazio if q.viu_filho || q.viu_caracteres => {
                return Err(format!(
                    "cvc-complex-type.2.1: Element '{}' must have no character or element information item [children], because the type's content type is empty.",
                    q.nome
                ));
            }
            Conteudo::Simples(s) => {
                if q.viu_filho {
                    return Err(format!(
                        "cvc-complex-type.2.2: Element '{}' must have no element [children], and the value must be valid.",
                        q.nome
                    ));
                }
                if let Some(st) = self.simples(*s) {
                    let normalizado = st.espacos.normalizar(&self.conteudo_texto(q));
                    self.validar_valor(st, &normalizado, &normalizado)?;
                    self.verificar_fixo(q, st, &normalizado)?;
                }
            }
            Conteudo::Elementos(_) if q.viu_caracteres => {
                return Err(format!(
                    "cvc-complex-type.2.3: Element '{}' cannot have character [children], because the type's content type is element-only.",
                    q.nome
                ));
            }
            _ => {}
        }
        let Some((dfa, estado)) = &q.modelo else {
            return Ok(());
        };
        if estado.atual.is_none() || dfa.completo(estado) {
            return Ok(());
        }
        let (_, esperado) = self.esperados(dfa, estado);
        match dfa.ocorrencia(estado) {
            Some(o) if estado.contador < o.min => {
                Err(self.minimo_nao_atingido(&q.nome, &dfa.termos[o.termo], o.min, estado.contador))
            }
            _ => Err(format!(
                "cvc-complex-type.2.4.b: The content of element '{}' is not complete. One of '{esperado}' is expected.",
                q.nome
            )),
        }
    }
}

fn atributo_xsi<'a>(e: &'a Elemento, local: &str) -> Option<&'a Atributo> {
    e.atributos
        .iter()
        .find(|a| a.nome.uri.as_deref() == Some(NS_XSI) && a.nome.local == local)
}

impl Manipulador for Validador<'_> {
    fn inicio(&mut self, e: &Elemento, ns: &Namespaces, pos: Posicao) -> Result<(), ErroXml> {
        self.iniciar(e, ns).map_err(|mensagem| ErroXml {
            posicao: pos,
            mensagem,
        })
    }

    fn texto(&mut self, t: &str) {
        let esquema = self.esquema;
        let Some(q) = self.pilha.last_mut() else {
            return;
        };
        if q.pulados.is_some() {
            return;
        }
        q.viu_texto |= !t.is_empty();
        q.viu_caracteres |= t.chars().any(|c| !matches!(c, ' ' | '\t' | '\n' | '\r'));
        let guarda_texto = match esquema.tipo(q.tipo) {
            Tipo::Simples(_) => true,
            Tipo::Complexo(c) => matches!(c.conteudo, Conteudo::Simples(_)),
        };
        if guarda_texto {
            q.texto.push_str(t);
        }
    }

    fn fim(&mut self, pos: Posicao) -> Result<(), ErroXml> {
        self.finalizar().map_err(|mensagem| ErroXml {
            posicao: pos,
            mensagem,
        })
    }
}
