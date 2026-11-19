"""junta casos (pedidos) e respostas gravadas da versão Java em fixtures
gzip para o teste de paridade do rtc-server (server/tests/paridade.rs)

uso: python3 gerar_fixtures.py DIR_GRAVACOES DIR_SAIDA
DIR_GRAVACOES tem pares casos_<suite>.json / golden_<suite>.json gerados
pelo gravar.py contra o container Java"""
import gzip, json, sys, zlib
from pathlib import Path

# corpo esperado acima disso vai como crc32 + tamanho
LIMITE_CORPO = 64 * 1024
# suítes gravadas com SEM_REDIRECT=1
SEM_REDIRECIONAR = {'swagger'}


def esperado(g):
    e = {'status': g['status'], 'cabecalhos': g['headers']}
    corpo = g['body'].encode('utf-8', 'surrogatepass')
    if len(corpo) > LIMITE_CORPO:
        e['corpo_crc32'] = zlib.crc32(corpo)
        e['corpo_tamanho'] = len(corpo)
    else:
        e['corpo'] = g['body']
    return e


def caso(c, g):
    saida = {'id': c['id'], 'metodo': c['method'], 'caminho': c['path'], 'cabecalhos': c['headers']}
    if c.get('body_hex'):
        saida['corpo_hex'] = c['body_hex']
    elif c['body'] is not None:
        saida['corpo_hex'] = c['body'].encode('utf-8', 'surrogatepass').hex()
    saida['esperado'] = esperado(g)
    return saida


def main():
    origem, destino = Path(sys.argv[1]), Path(sys.argv[2])
    for arq in sorted(origem.glob('casos_*.json')):
        suite = arq.stem.removeprefix('casos_')
        gravadas = origem / f'golden_{suite}.json'
        if not gravadas.exists():
            continue
        respostas = {g['id']: g for g in json.loads(gravadas.read_text())}
        casos = [caso(c, respostas[c['id']]) for c in json.loads(arq.read_text()) if c['id'] in respostas]
        doc = {'seguir_redirecionamento': suite not in SEM_REDIRECIONAR, 'casos': casos}
        dados = json.dumps(doc, ensure_ascii=True, separators=(',', ':')).encode()
        (destino / f'{suite}.json.gz').write_bytes(gzip.compress(dados, mtime=0))
        print(suite, len(casos))


main()
