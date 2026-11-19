"""compara respostas gravadas (golden vs nossa) com normalização de ordem aleatória"""
import json, re, sys
from collections import Counter

def carregar(p):
    return {x['id']: x for x in json.load(open(p))}

def norm_bv(detalhe):
    # Bean Validation: ordem das violações é aleatória
    m = re.match(r'(Um ou mais campos são inválidos - )(.*)$', detalhe, re.S)
    if not m:
        return detalhe
    itens = re.findall(r'\[[^\]]*?: .*?\] ', m.group(2))
    return m.group(1) + ''.join(sorted(itens))

def norm(o):
    if isinstance(o, dict):
        d = {k: norm(v) for k, v in o.items()}
        # ElementoBaseCalculo.todos() é Set.of: ordem muda a cada JVM
        if isinstance(d.get('avisos'), list) and all(isinstance(a, str) and a.startswith('Campo ') for a in d['avisos']):
            d['avisos'] = sorted(d['avisos'])
        if isinstance(d.get('detail'), str):
            d['detail'] = norm_bv(d['detail'])
        return d
    if isinstance(o, list):
        l = [norm(v) for v in o]
        if l and all(isinstance(v, dict) and 'pointer' in v for v in l):
            l = sorted(l, key=lambda v: json.dumps(v, sort_keys=True, ensure_ascii=False))
        return l
    return o

def corpo(b):
    try:
        return json.dumps(norm(json.loads(b)), ensure_ascii=False)
    except Exception:
        return b

IGNORAR = {'date'}
def cab(h):
    return Counter((k, v) for k, v in h if k not in IGNORAR)

g, r = carregar(sys.argv[1]), carregar(sys.argv[2])
filtro = sys.argv[3] if len(sys.argv) > 3 else ''
ok = falhas = 0
for i, x in g.items():
    if filtro and not re.search(filtro, i):
        continue
    y = r.get(i)
    if y is None:
        continue
    erros = []
    if x['status'] != y['status']:
        erros.append(f"status {x['status']} != {y['status']}")
    if cab(x['headers']) != cab(y['headers']):
        erros.append(f"headers -{dict(cab(x['headers'])-cab(y['headers']))} +{dict(cab(y['headers'])-cab(x['headers']))}")
    bx, by = corpo(x['body']), corpo(y['body'])
    if bx != by:
        k = next((n for n in range(min(len(bx), len(by))) if bx[n] != by[n]), min(len(bx), len(by)))
        erros.append(f"body @{k}: java=…{bx[max(0,k-80):k+120]!r}\n      rust=…{by[max(0,k-80):k+120]!r}")
    if erros:
        falhas += 1
        print('FAIL', i)
        for e in erros:
            print('   ', e)
    else:
        ok += 1
print(f'ok={ok} falhas={falhas}')
