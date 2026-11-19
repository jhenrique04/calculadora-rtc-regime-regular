"""casos HTTP equivalentes aos testes Java da base unificada (BaseCalculoUnificada*Test)"""
import json, re
from pathlib import Path
JAVA = Path(__file__).resolve().parents[5] / 'calculadora-rtc-regime-regular'
T = JAVA / 'src/test/java'
URL = '/api/calculadora/base-calculo/base-calculo-unificada'
COMP = ['vProd','vFrete','vSeg','vOutro','vII','vIS','vDesc','vPIS','vCOFINS','vICMS','vICMSUFDest','vFCP','vFCPUFDest','vICMSMono','vISSQN']
def num(e):
    e = e.strip()
    m = re.fullmatch(r'new BigDecimal\("([^"]+)"\)', e)
    return m.group(1) if m else ('null' if e == 'null' else e)
def args(texto):
    saida, nivel, atual = [], 0, ''
    for c in texto:
        if c == ',' and nivel == 0: saida.append(atual); atual = ''; continue
        nivel += c in '(' ; nivel -= c in ')'; atual += c
    return saida + [atual]
def corpo(campos): return '{' + ','.join(f'"{k}":{v}' for k, v in campos.items()) + '}'
casos, esperados = [], {}
def emitir(nome, tnome, campos, n):
    cid = f'un/{nome}/{tnome}#{n}'
    casos.append({'id': cid, 'method': 'POST', 'path': URL, 'headers': {'Content-Type': 'application/json'}, 'body': corpo(campos)})
    return cid
for nome in ('BaseCalculoUnificadaServiceTest', 'BaseCalculoUnificadaInputTest'):
    fonte = next(T.rglob(nome + '.java')).read_text(encoding='utf-8')
    for teste in re.split(r'@Test\b', fonte)[1:]:
        tnome = re.search(r'void (\w+)\(', teste).group(1)
        comps, dto, n, ids = {}, {}, 0, {}
        ultimo = None
        for inst in teste.split(';'):
            m = re.search(r'(\w+) = comp\((.*)\)\s*$', inst, re.S)
            if m: comps[m.group(1)] = dict(zip(COMP, map(num, args(m.group(2)))))
            m = re.search(r'input\.set(AnoFatoGerador|TipoDocumento|Natureza)\((.*)\)', inst)
            if m: dto[m.group(1)[0].lower() + m.group(1)[1:]] = '"%s"' % m.group(2).split('.')[-1] if m.group(1) == 'Natureza' else m.group(2)
            m = re.search(r'getComponentes\(\)\.set(\w+)\((.*)\)\s*$', inst, re.S)
            if m: dto[next(c for c in COMP if c.lower() == m.group(1).lower())] = num(m.group(2))
            var = re.match(r'\s*(?:\w+\s+)?(\w+)\s*=', inst)
            m = re.search(r'service\.calcular\((\w+), (\d+), BaseCalculoNatureza\.(\w+), (\w+)\)', inst)
            if m:
                n += 1
                campos = {'anoFatoGerador': m.group(2), 'tipoDocumento': m.group(4), 'natureza': f'"{m.group(3)}"'}
                campos.update({k: v for k, v in comps[m.group(1)].items() if v != 'null'})
                ultimo = emitir(nome, tnome, campos, n)
            m = re.search(r'service\.calcular\(input\)', inst)
            if m:
                n += 1; ultimo = emitir(nome, tnome, dict(dto), n)
            m = re.search(r'input\((\d+), (\d+), "(\w+)",\s*Map\.(?:of|ofEntries)\((.*)\)\)', inst, re.S)
            if m:
                n += 1
                campos = {'anoFatoGerador': m.group(1), 'tipoDocumento': m.group(2), 'natureza': f'"{m.group(3)}"'}
                campos.update(dict(re.findall(r'"(\w+)",\s*"([^"]+)"', m.group(4))))
                ultimo = emitir(nome, tnome, campos, n)
            if ultimo and var: ids[var.group(1)] = ultimo
            for v, campo, val in re.findall(r'assertThat\((\w+)(?:\.(getBase\w+)\(\))?\)\.isEqualByComparingTo\("([^"]+)"\)', inst):
                alvo = ids.get(v, ultimo)
                campo = campo or ('getBaseRG' if 'getBaseRG' in teste.split(v)[0][-200:] else None)
                if campo: esperados.setdefault(alvo, []).append((campo[3].lower() + campo[4:], val))
json.dump(casos, open('casos_unificada.json', 'w'), indent=0)
json.dump(esperados, open('esperados_unificada.json', 'w'))
print(len(casos), 'casos', sum(map(len, esperados.values())), 'valores afirmados')
