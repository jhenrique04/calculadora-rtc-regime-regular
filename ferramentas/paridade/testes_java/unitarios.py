"""casos HTTP equivalentes aos testes unitários Java de base de cálculo:
reproduz os setters até cada chamada do serviço e guarda o valor afirmado"""
import json, re
from pathlib import Path
JAVA = Path(__file__).resolve().parents[5] / 'calculadora-rtc-regime-regular'
T = JAVA / 'src/test/java'
SCHEMAS = json.loads((Path(__file__).resolve().parents[3] / 'server/assets/springdoc/api-docs.json').read_text())['components']['schemas']
ALVOS = {
    'Teste_BaseCalculoService_CalcularCibs': ('calcularCibs', '/api/calculadora/base-calculo/cbs-ibs-mercadorias', 'BaseCalculoCibsInput'),
    'Teste_BaseCalculoService_CalcularISMercadorias': ('calcularISMercadorias', '/api/calculadora/base-calculo/is-mercadorias', 'BaseCalculoISMercadoriasInput'),
    'Teste_NfseService_CalcularBaseCalculo': ('calcularBaseCalculo', '/api/calculadora/nfse/base-calculo', 'NfseBaseCalculoInput'),
}
def valor(expr):
    expr = expr.strip()
    m = re.fullmatch(r'new BigDecimal\("([^"]+)"\)', expr)
    if m: return m.group(1)
    m = re.fullmatch(r'BigDecimal\.valueOf\(([-\d.]+)L?\)', expr)
    if m: return m.group(1)
    if expr == 'BigDecimal.ZERO': return '0'
    if expr == 'BigDecimal.ONE': return '1'
    if expr == 'BigDecimal.TEN': return '10'
    if expr == 'null': return 'null'
    if re.fullmatch(r'-?\d+', expr): return expr
    raise ValueError(expr)
def corpo(campos):
    return '{' + ','.join(f'"{k}":{v}' for k, v in campos.items()) + '}'
casos, esperados = [], {}
for nome, (metodo, url, schema) in ALVOS.items():
    props = {p.lower(): p for p in SCHEMAS[schema]['properties']}
    fonte = (next(T.rglob(nome + '.java'))).read_text(encoding='utf-8')
    for teste in re.split(r'@Test\b', fonte)[1:]:
        tnome = re.search(r'void (\w+)\(', teste).group(1)
        campos, n, ultimo = {}, 0, None
        for instrucao in teste.split(';'):
            if re.search(r'new \w+Input\(\)', instrucao): campos = {}
            m = re.search(r'input\.set(\w+)\((.*)\)\s*$', instrucao, re.S)
            if m: campos[props[m.group(1).lower()]] = valor(m.group(2))
            if 'service.' + metodo + '(input)' in instrucao:
                n += 1
                ultimo = f'un/{nome.split("_",1)[1]}/{tnome}#{n}'
                casos.append({'id': ultimo, 'method': 'POST', 'path': url,
                              'headers': {'Content-Type': 'application/json'}, 'body': corpo(campos)})
            a = re.search(r'assertEquals\(0, new BigDecimal\("([^"]+)"\)\.compareTo', instrucao)
            if a and ultimo: esperados.setdefault(ultimo, a.group(1))
json.dump(casos, open('casos_unitarios.json', 'w'), ensure_ascii=False, indent=0)
json.dump(esperados, open('esperados_unitarios.json', 'w'))
print(len(casos), 'casos', len(esperados), 'com valor afirmado')
