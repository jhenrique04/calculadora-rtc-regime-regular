//! modelo de conteúdo como o `XSDFACM` do Xerces: árvore de partículas,
//! construção por followpos e estados de contagem para `{n,m}` compactos.
//! Os estados são construídos sob demanda: `{0,5000}` fora do modo compacto
//! gera milhares de estados, mas um documento só visita alguns

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use super::{Compositor, Curinga, IdElemento, Particula, TermoParticula};

#[derive(Debug, Clone)]
pub enum Termo {
    Elemento(IdElemento),
    Curinga(Arc<Curinga>),
}

impl Termo {
    fn mesmo(&self, outro: &Self) -> bool {
        match (self, outro) {
            (Self::Elemento(a), Self::Elemento(b)) => a == b,
            (Self::Curinga(a), Self::Curinga(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ocorrencia {
    pub min: u32,
    pub max: Option<u32>,
    pub termo: usize,
}

/// O `int[3]` do Xerces: estado atual (`None` = FIRST_ERROR), anterior e contador
#[derive(Debug, Clone, Copy)]
pub struct EstadoCm {
    pub atual: Option<usize>,
    pub anterior: usize,
    pub contador: u32,
}

#[derive(Debug)]
struct Linha {
    destinos: Vec<Option<usize>>,
    final_: bool,
    ocorrencia: Option<Ocorrencia>,
}

#[derive(Debug, Default)]
struct Estados {
    conjuntos: Vec<Conjunto>,
    indice: HashMap<Conjunto, usize>,
    linhas: Vec<Option<Arc<Linha>>>,
}

impl Estados {
    fn id(&mut self, conjunto: Conjunto) -> usize {
        if let Some(&id) = self.indice.get(&conjunto) {
            return id;
        }
        let id = self.conjuntos.len();
        self.indice.insert(conjunto.clone(), id);
        self.conjuntos.push(conjunto);
        self.linhas.push(None);
        id
    }
}

#[derive(Debug)]
pub struct Dfa {
    pub termos: Vec<Termo>,
    folhas: Vec<Folha>,
    seguintes: Vec<Conjunto>,
    fim: usize,
    estados: Mutex<Estados>,
}

impl Dfa {
    pub fn construir(particula: Option<&Particula>) -> Self {
        let mut c = Construtor::default();
        let raiz = particula.and_then(|p| {
            if usa_folhas_repetidas(p) {
                c.compacta(p)
            } else {
                c.completa(p)
            }
        });
        c.dfa(raiz)
    }

    pub fn inicio(&self) -> EstadoCm {
        EstadoCm {
            atual: Some(0),
            anterior: 0,
            contador: 0,
        }
    }

    fn linha(&self, estado: usize) -> Arc<Linha> {
        let mut g = self.estados.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(Some(l)) = g.linhas.get(estado) {
            return Arc::clone(l);
        }
        let conjunto = g.conjuntos[estado].clone();
        let n = self.fim + 1;
        let mut novos = vec![Conjunto::novo(n); self.termos.len()];
        for p in conjunto.posicoes().filter(|p| *p != self.fim) {
            novos[self.folhas[p].termo].unir(&self.seguintes[p]);
        }
        let destinos: Vec<Option<usize>> = novos
            .into_iter()
            .map(|novo| (!novo.vazio()).then(|| g.id(novo)))
            .collect();
        let linha = Arc::new(Linha {
            final_: conjunto.contem(self.fim),
            ocorrencia: self.ocorrencia_do_laco(estado, &destinos),
            destinos,
        });
        g.linhas[estado] = Some(Arc::clone(&linha));
        linha
    }

    /// estado com laço num termo de folha repetida `{n,m}` conta as voltas
    fn ocorrencia_do_laco(&self, estado: usize, destinos: &[Option<usize>]) -> Option<Ocorrencia> {
        let termo = destinos.iter().position(|d| *d == Some(estado))?;
        let (min, max) = self
            .folhas
            .iter()
            .filter(|f| f.termo == termo)
            .find_map(|f| f.repeticao)?;
        Some(Ocorrencia { min, max, termo })
    }

    fn falhar(e: &mut EstadoCm, atual: usize) {
        e.anterior = atual;
        e.atual = None;
    }

    fn entrar(&self, e: &mut EstadoCm, termo: usize, proximo: usize) {
        if let Some(o) = self.linha(proximo).ocorrencia {
            e.contador = u32::from(termo == o.termo);
        }
    }

    /// `oneTransition`: devolve o índice do termo que casou
    pub fn transicao(&self, e: &mut EstadoCm, casa: impl Fn(&Termo) -> bool) -> Option<usize> {
        let atual = e.atual?;
        let linha = self.linha(atual);
        let achado = linha
            .destinos
            .iter()
            .enumerate()
            .find_map(|(t, d)| d.filter(|_| casa(&self.termos[t])).map(|p| (t, p)));
        let Some((termo, proximo)) = achado else {
            Self::falhar(e, atual);
            return None;
        };
        match linha.ocorrencia {
            Some(o) if atual == proximo => {
                e.contador += 1;
                if o.max.is_some_and(|m| e.contador > m) {
                    return self.outro_termo(e, &linha, atual, termo, &casa);
                }
            }
            Some(o) if e.contador < o.min => {
                Self::falhar(e, atual);
                return None;
            }
            _ => self.entrar(e, termo, proximo),
        }
        e.atual = Some(proximo);
        Some(termo)
    }

    /// `findMatchingElemDecl`: outro termo com o mesmo nome depois de estourar o máximo
    fn outro_termo(
        &self,
        e: &mut EstadoCm,
        linha: &Linha,
        atual: usize,
        depois: usize,
        casa: &impl Fn(&Termo) -> bool,
    ) -> Option<usize> {
        for t in depois + 1..linha.destinos.len() {
            if let Some(p) = linha.destinos[t].filter(|_| casa(&self.termos[t])) {
                e.atual = Some(p);
                self.entrar(e, t, p);
                return Some(t);
            }
        }
        Self::falhar(e, atual);
        None
    }

    fn estado_referencia(e: &EstadoCm) -> usize {
        e.atual.unwrap_or(e.anterior)
    }

    pub fn esperados(&self, e: &EstadoCm) -> Vec<usize> {
        let atual = Self::estado_referencia(e);
        let linha = self.linha(atual);
        let mut saida = Vec::new();
        for (t, destino) in linha.destinos.iter().enumerate() {
            let Some(destino) = *destino else { continue };
            if let Some(o) = linha.ocorrencia {
                if destino == atual {
                    if o.max.is_some_and(|m| e.contador >= m) {
                        continue;
                    }
                } else if e.contador < o.min {
                    continue;
                }
            }
            saida.push(t);
        }
        saida
    }

    pub fn ocorrencia(&self, e: &EstadoCm) -> Option<Ocorrencia> {
        self.linha(Self::estado_referencia(e)).ocorrencia
    }

    pub fn completo(&self, e: &EstadoCm) -> bool {
        let Some(atual) = e.atual else { return false };
        let linha = self.linha(atual);
        linha.final_ && linha.ocorrencia.is_none_or(|o| e.contador >= o.min)
    }
}

fn usa_folhas_repetidas(p: &Particula) -> bool {
    let TermoParticula::Grupo(_, filhos) = &p.termo else {
        return true;
    };
    if p.min != 1 || p.max != Some(1) {
        return match filhos.as_slice() {
            [] => true,
            [f] => !matches!(f.termo, TermoParticula::Grupo(..)) && f.min == 1 && f.max == Some(1),
            _ => false,
        };
    }
    filhos.iter().all(usa_folhas_repetidas)
}

#[derive(Debug, Clone)]
enum No {
    Folha(usize),
    Seq(Box<No>, Box<No>),
    Alt(Box<No>, Box<No>),
    Opcional(Box<No>),
    Estrela(Box<No>),
    Mais(Box<No>),
}

impl No {
    fn binario(c: Compositor, a: No, b: No) -> No {
        match c {
            Compositor::Sequencia => No::Seq(Box::new(a), Box::new(b)),
            Compositor::Escolha => No::Alt(Box::new(a), Box::new(b)),
        }
    }
}

#[derive(Debug)]
struct Folha {
    termo: usize,
    repeticao: Option<(u32, Option<u32>)>,
}

#[derive(Default)]
struct Construtor {
    termos: Vec<Termo>,
    folhas: Vec<Folha>,
}

/// conjunto de posições em bits
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Conjunto(Vec<u64>);

const BITS: usize = 64;

impl Conjunto {
    fn novo(tamanho: usize) -> Self {
        Self(vec![0; tamanho.div_ceil(BITS)])
    }

    fn com(tamanho: usize, p: usize) -> Self {
        let mut c = Self::novo(tamanho);
        c.inserir(p);
        c
    }

    fn inserir(&mut self, p: usize) {
        self.0[p / BITS] |= 1 << (p % BITS);
    }

    fn contem(&self, p: usize) -> bool {
        self.0[p / BITS] & (1 << (p % BITS)) != 0
    }

    fn unir(&mut self, outro: &Self) {
        for (a, b) in self.0.iter_mut().zip(&outro.0) {
            *a |= b;
        }
    }

    fn vazio(&self) -> bool {
        self.0.iter().all(|w| *w == 0)
    }

    fn posicoes(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().enumerate().flat_map(|(k, &w)| {
            (0..BITS)
                .filter(move |b| w & (1 << b) != 0)
                .map(move |b| k * BITS + b)
        })
    }
}

struct Info {
    anulavel: bool,
    primeiros: Conjunto,
    ultimos: Conjunto,
}

impl Construtor {
    fn indice_termo(&mut self, t: &TermoParticula) -> usize {
        let termo = match t {
            TermoParticula::Elemento(id) => Termo::Elemento(*id),
            TermoParticula::Curinga(c) => Termo::Curinga(Arc::clone(c)),
            TermoParticula::Grupo(..) => return usize::MAX,
        };
        if let Some(k) = self.termos.iter().position(|x| x.mesmo(&termo)) {
            return k;
        }
        self.termos.push(termo);
        self.termos.len() - 1
    }

    fn folha(&mut self, t: &TermoParticula, repeticao: Option<(u32, Option<u32>)>) -> No {
        let termo = self.indice_termo(t);
        self.folhas.push(Folha { termo, repeticao });
        No::Folha(self.folhas.len() - 1)
    }

    fn compacta(&mut self, p: &Particula) -> Option<No> {
        match &p.termo {
            TermoParticula::Grupo(c, filhos) => {
                if let [unico] = filhos.as_slice()
                    && (p.min != 1 || p.max != Some(1))
                {
                    return Some(self.compacta_folha(&unico.termo, p.min, p.max));
                }
                let mut nos = Vec::new();
                for f in filhos {
                    if let Some(n) = self.compacta(f) {
                        nos.push(n);
                    }
                }
                self.juntar(*c, nos, filhos.len())
            }
            t => Some(self.compacta_folha(t, p.min, p.max)),
        }
    }

    fn compacta_folha(&mut self, t: &TermoParticula, min: u32, max: Option<u32>) -> No {
        match (min, max) {
            (1, Some(1)) => self.folha(t, None),
            (0, Some(1)) => No::Opcional(Box::new(self.folha(t, None))),
            (0, None) => No::Estrela(Box::new(self.folha(t, None))),
            (1, None) => No::Mais(Box::new(self.folha(t, None))),
            _ => {
                let f = Box::new(self.folha(t, Some((min, max))));
                if min == 0 {
                    No::Estrela(f)
                } else {
                    No::Mais(f)
                }
            }
        }
    }

    fn juntar(&self, c: Compositor, nos: Vec<No>, total: usize) -> Option<No> {
        let quantos = nos.len();
        let mut no = nos.into_iter().reduce(|a, b| No::binario(c, a, b))?;
        if c == Compositor::Escolha && quantos < total {
            no = No::Opcional(Box::new(no));
        }
        Some(no)
    }

    /// `buildSyntaxTree` sem a otimização de UPA: `{n,m}` vira cópias
    fn completa(&mut self, p: &Particula) -> Option<No> {
        let no = match &p.termo {
            TermoParticula::Grupo(c, filhos) => {
                let mut nos = Vec::new();
                for f in filhos {
                    if let Some(n) = self.completa(f) {
                        nos.push(n);
                    }
                }
                self.juntar(*c, nos, filhos.len())?
            }
            t => self.folha(t, None),
        };
        let curinga = matches!(p.termo, TermoParticula::Curinga(_));
        self.expandir(no, p.min, p.max, curinga)
    }

    fn expandir(&mut self, no: No, min: u32, max: Option<u32>, curinga: bool) -> Option<No> {
        Some(match (min, max) {
            (_, Some(0)) => return None,
            (1, Some(1)) => no,
            (0, Some(1)) => No::Opcional(Box::new(no)),
            (0, None) => No::Estrela(Box::new(no)),
            (1, None) => No::Mais(Box::new(no)),
            // precedência de operadores do Xerces: curinga {n,m} perde os limites
            _ if curinga => {
                if min == 0 {
                    No::Estrela(Box::new(no))
                } else {
                    No::Mais(Box::new(no))
                }
            }
            (n, None) => {
                let copias = self.multiplos(&no, n - 1, true)?;
                No::Seq(Box::new(copias), Box::new(No::Mais(Box::new(no))))
            }
            (n, Some(m)) => {
                let obrigatorios = if n > 0 {
                    self.multiplos(&no, n, false)
                } else {
                    None
                };
                if m <= n {
                    return obrigatorios;
                }
                let opcional = No::Opcional(Box::new(no));
                match obrigatorios {
                    None => self.multiplos(&opcional, m - n, false)?,
                    Some(o) => {
                        let resto = self.multiplos(&opcional, m - n, true)?;
                        No::Seq(Box::new(o), Box::new(resto))
                    }
                }
            }
        })
    }

    fn multiplos(&mut self, no: &No, n: u32, copiar_primeiro: bool) -> Option<No> {
        match n {
            0 => None,
            1 if copiar_primeiro => Some(self.copiar(no)),
            1 => Some(no.clone()),
            _ => {
                let metade = n / 2;
                let a = self.multiplos(no, metade, copiar_primeiro)?;
                let b = self.multiplos(no, n - metade, true)?;
                Some(No::Seq(Box::new(a), Box::new(b)))
            }
        }
    }

    /// cópia com posições novas para cada folha
    fn copiar(&mut self, no: &No) -> No {
        match no {
            No::Folha(k) => {
                let termo = self.folhas[*k].termo;
                self.folhas.push(Folha {
                    termo,
                    repeticao: None,
                });
                No::Folha(self.folhas.len() - 1)
            }
            No::Seq(a, b) => No::Seq(Box::new(self.copiar(a)), Box::new(self.copiar(b))),
            No::Alt(a, b) => No::Alt(Box::new(self.copiar(a)), Box::new(self.copiar(b))),
            No::Opcional(a) => No::Opcional(Box::new(self.copiar(a))),
            No::Estrela(a) => No::Estrela(Box::new(self.copiar(a))),
            No::Mais(a) => No::Mais(Box::new(self.copiar(a))),
        }
    }

    fn analisar(&self, no: &No, n: usize, seguintes: &mut [Conjunto]) -> Info {
        match no {
            No::Folha(k) => Info {
                anulavel: false,
                primeiros: Conjunto::com(n, *k),
                ultimos: Conjunto::com(n, *k),
            },
            No::Seq(a, b) => {
                let ia = self.analisar(a, n, seguintes);
                let ib = self.analisar(b, n, seguintes);
                for p in ia.ultimos.posicoes() {
                    seguintes[p].unir(&ib.primeiros);
                }
                let mut primeiros = ia.primeiros;
                if ia.anulavel {
                    primeiros.unir(&ib.primeiros);
                }
                let mut ultimos = ib.ultimos;
                if ib.anulavel {
                    ultimos.unir(&ia.ultimos);
                }
                Info {
                    anulavel: ia.anulavel && ib.anulavel,
                    primeiros,
                    ultimos,
                }
            }
            No::Alt(a, b) => {
                let mut ia = self.analisar(a, n, seguintes);
                let ib = self.analisar(b, n, seguintes);
                ia.primeiros.unir(&ib.primeiros);
                ia.ultimos.unir(&ib.ultimos);
                Info {
                    anulavel: ia.anulavel || ib.anulavel,
                    ..ia
                }
            }
            No::Opcional(a) => Info {
                anulavel: true,
                ..self.analisar(a, n, seguintes)
            },
            No::Estrela(a) | No::Mais(a) => {
                let ia = self.analisar(a, n, seguintes);
                for p in ia.ultimos.posicoes() {
                    seguintes[p].unir(&ia.primeiros);
                }
                Info {
                    anulavel: matches!(no, No::Estrela(_)) || ia.anulavel,
                    ..ia
                }
            }
        }
    }

    fn dfa(self, raiz: Option<No>) -> Dfa {
        let fim = self.folhas.len();
        let n = fim + 1;
        let mut seguintes = vec![Conjunto::novo(n); n];
        let inicial = match &raiz {
            None => Conjunto::com(n, fim),
            Some(r) => {
                let info = self.analisar(r, n, &mut seguintes);
                for p in info.ultimos.posicoes() {
                    seguintes[p].inserir(fim);
                }
                let mut ini = info.primeiros;
                if info.anulavel {
                    ini.inserir(fim);
                }
                ini
            }
        };
        let mut estados = Estados::default();
        estados.id(inicial);
        Dfa {
            termos: self.termos,
            folhas: self.folhas,
            seguintes,
            fim,
            estados: Mutex::new(estados),
        }
    }
}
