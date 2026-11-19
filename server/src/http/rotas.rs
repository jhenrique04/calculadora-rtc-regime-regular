//! tabela de `@RequestMapping` e casamento de caminho como o `PathPattern`:
//! segmentos literais vencem variáveis

use hyper::Method;

use super::App;
use super::erro::ErroHttp;
use super::pedido::Pedido;
use super::resposta::Resposta;

/// variáveis de caminho capturadas (`{nome}` → valor)
pub type Variaveis = Vec<(&'static str, String)>;

pub type Manipulador = fn(&App, &Pedido<'_>) -> Result<Resposta, ErroHttp>;

pub struct Rota {
    pub padrao: &'static str,
    pub metodo: Method,
    /// `consumes` do mapeamento, se houver
    pub consome: Option<&'static str>,
    /// tipo produzido, para a negociação com `Accept`
    pub produz: &'static str,
    pub manipulador: Manipulador,
}

pub struct Casamento<'r> {
    pub rotas: Vec<&'r Rota>,
    pub variaveis: Variaveis,
}

fn segmentos(caminho: &str) -> Vec<&str> {
    caminho.trim_start_matches('/').split('/').collect()
}

/// quantidade de literais (para desempate) e variáveis capturadas
fn casar(padrao: &'static str, caminho: &[String]) -> Option<(usize, Variaveis)> {
    let partes = segmentos(padrao);
    if partes.len() != caminho.len() {
        return None;
    }
    let mut literais = 0;
    let mut vars = Vec::new();
    for (p, c) in partes.into_iter().zip(caminho) {
        match p.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
            Some(nome) if !c.is_empty() => vars.push((nome, c.clone())),
            Some(_) => return None,
            None if p == c => literais += 1,
            None => return None,
        }
    }
    Some((literais, vars))
}

/// rotas do padrão mais específico que casa com o caminho (segmentos já decodificados)
pub fn resolver<'r>(tabela: &'r [Rota], caminho: &[String]) -> Option<Casamento<'r>> {
    let mut melhor: Option<(usize, &'static str, Variaveis)> = None;
    for r in tabela {
        if let Some((literais, vars)) = casar(r.padrao, caminho)
            && melhor.as_ref().is_none_or(|(l, _, _)| literais > *l)
        {
            melhor = Some((literais, r.padrao, vars));
        }
    }
    let (_, padrao, variaveis) = melhor?;
    Some(Casamento {
        rotas: tabela.iter().filter(|r| r.padrao == padrao).collect(),
        variaveis,
    })
}

/// `Allow` do `HttpOptionsHandler`: métodos mapeados, HEAD junto de GET, e OPTIONS
pub fn metodos_options(rotas: &[&Rota]) -> String {
    let mut m: Vec<&str> = Vec::new();
    for r in rotas {
        m.push(r.metodo.as_str());
        if r.metodo == Method::GET {
            m.push("HEAD");
        }
    }
    m.push("OPTIONS");
    sem_repetidos(m).join(",")
}

/// `HttpHeaders.setAllow` do 405 separa com ", "
pub fn metodos_permitidos(rotas: &[&Rota]) -> String {
    sem_repetidos(rotas.iter().map(|r| r.metodo.as_str()).collect()).join(", ")
}

// o Spring percorre um HashMap de mapeamentos; a ordem da tabela reproduz a observada
fn sem_repetidos(m: Vec<&str>) -> Vec<&str> {
    let mut saida: Vec<&str> = Vec::with_capacity(m.len());
    for x in m {
        if !saida.contains(&x) {
            saida.push(x);
        }
    }
    saida
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::resposta::Resposta;
    use hyper::StatusCode;

    fn nada(_: &App, _: &Pedido<'_>) -> Result<Resposta, ErroHttp> {
        Ok(Resposta::new(StatusCode::OK))
    }

    fn rota(padrao: &'static str, metodo: Method) -> Rota {
        Rota {
            padrao,
            metodo,
            consome: None,
            produz: crate::http::midia::JSON,
            manipulador: nada,
        }
    }

    fn seg(s: &str) -> Vec<String> {
        segmentos(s).into_iter().map(str::to_string).collect()
    }

    #[test]
    fn literal_vence_variavel() {
        let t = [
            rota("/c/{id}", Method::GET),
            rota("/c/cbs-ibs", Method::GET),
            rota("/c/cbs-ibs", Method::POST),
        ];
        let Some(c) = resolver(&t, &seg("/c/cbs-ibs")) else {
            panic!("sem casamento")
        };
        assert_eq!(c.rotas.len(), 2);
        assert!(c.variaveis.is_empty());
        let Some(c) = resolver(&t, &seg("/c/7")) else {
            panic!("sem casamento")
        };
        assert_eq!(c.variaveis, vec![("id", "7".to_string())]);
        assert_eq!(metodos_options(&c.rotas), "GET,HEAD,OPTIONS");
        assert!(resolver(&t, &seg("/c/")).is_none());
    }
}
