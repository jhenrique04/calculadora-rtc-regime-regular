"""pedidos dos testes HTTP Java (MockMvc) capturados pelo CapturaRequisicoes
viram casos para o gravar.py, sem repetição

captura: docker build -t rtc-java-testes . (FROM rtc-java-ref, roda mvn test)
         docker create --name c rtc-java-testes && docker cp c:/app/captura.jsonl .
uso: python3 captura_para_casos.py captura.jsonl casos_testes.json"""
import json, sys
from collections import Counter

# o cliente de gravação gera os próprios
IGNORADOS = {'host', 'content-length'}


def cabecalhos(pares):
    saida = {}
    for k, v in pares:
        if k.lower() not in IGNORADOS:
            saida[k] = v if k not in saida else saida[k] + ', ' + v
    return saida


def main():
    origem, destino = sys.argv[1], sys.argv[2]
    casos, vistos, por_teste = [], set(), Counter()
    for linha in open(origem, encoding='utf-8'):
        c = json.loads(linha)
        caminho = c['uri'] if c['uri'].startswith('/api') else '/api' + c['uri']
        if c['consulta']:
            caminho += '?' + c['consulta']
        cab = cabecalhos(c['cabecalhos'])
        chave = (c['metodo'], caminho, json.dumps(cab, sort_keys=True), c['corpo_hex'])
        if chave in vistos:
            continue
        vistos.add(chave)
        nome = '.'.join(c['teste'].rsplit('.', 2)[-2:])
        por_teste[nome] += 1
        caso = {'id': f'jt/{nome}#{por_teste[nome]}', 'method': c['metodo'], 'path': caminho,
                'headers': cab, 'body': None}
        corpo = bytes.fromhex(c['corpo_hex'])
        if c['tem_corpo'] or corpo:
            caso['body'] = corpo.decode('utf-8')
        casos.append(caso)
    json.dump(casos, open(destino, 'w'), ensure_ascii=False, indent=0)
    print(len(casos), 'casos de', len(por_teste), 'testes')


if __name__ == '__main__':
    main()
