"""envia casos a um servidor e grava respostas normalizadas"""
import json, os, sys, urllib.request, urllib.error, gzip, re
base, entrada, saida = sys.argv[1], sys.argv[2], sys.argv[3]
casos = json.load(open(entrada))
GUARDAR = {'content-type', 'vary', 'x-calc-app-version', 'x-calc-db-version', 'x-warning-dados-simulados',
           'content-disposition', 'content-encoding', 'location', 'allow', 'access-control-allow-origin',
           'access-control-allow-methods', 'access-control-allow-headers', 'x-content-type-options', 'x-xss-protection',
           'cache-control', 'pragma', 'expires', 'x-frame-options', 'transfer-encoding', 'content-length',
           'content-range', 'accept-ranges', 'last-modified'}

class SemRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None

abrir = urllib.request.build_opener(SemRedirect).open if os.environ.get('SEM_REDIRECT') else urllib.request.urlopen
out = []
for c in casos:
    data = bytes.fromhex(c['body_hex']) if c.get('body_hex') else (c['body'].encode('utf-8', 'surrogatepass') if c['body'] is not None else None)
    req = urllib.request.Request(base + c['path'], data=data, method=c['method'], headers=c['headers'])
    try:
        r = abrir(req, timeout=60)
    except urllib.error.HTTPError as e:
        r = e
    corpo = r.read()
    headers = [(k.lower(), v) for k, v in r.headers.items() if k.lower() in GUARDAR]
    # data do jar/executável e boundary aleatório não são comparáveis
    headers = [(k, '<lm>' if k == 'last-modified' else v) for k, v in headers]
    texto = corpo.decode('utf-8', 'replace')
    for k, v in headers:
        m = re.search(r'boundary=(\S+)', v)
        if m:
            texto = texto.replace(m.group(1), '<b>')
    headers = [(k, re.sub(r'boundary=\S+', 'boundary=<b>', v)) for k, v in headers]
    texto = re.sub(r'"timestamp":"[^"]*"', '"timestamp":"<ts>"', texto)
    out.append({'id': c['id'], 'status': r.status, 'reason': r.reason, 'headers': headers, 'body': texto})
json.dump(out, open(saida, 'w'), ensure_ascii=False, indent=0)
print(len(out), 'respostas')
