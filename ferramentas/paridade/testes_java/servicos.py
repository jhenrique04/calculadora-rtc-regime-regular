"""casos HTTP equivalentes aos testes de serviço Java (Teste_tpdoc_*)"""
import json
from pathlib import Path
JAVA = Path(__file__).resolve().parents[5] / 'calculadora-rtc-regime-regular'
R = JAVA / 'src/test/resources/entradas/tpdoc'
casos = []
def caso(teste, arquivo, mut=None):
    op = json.loads((R / arquivo).read_text(encoding='utf-8'))
    if mut: mut(op)
    corpo = json.dumps(op, ensure_ascii=False)
    for pre, url in (('rg', '/api/calculadora/regime-geral'), ('obs', '/api/calculadora/observabilidade/regime-geral')):
        casos.append({'id': f'sv/{pre}/{teste}', 'method': 'POST', 'path': url,
                      'headers': {'Content-Type': 'application/json'}, 'body': corpo})
def item(**kv):
    def f(op):
        for k, v in kv.items():
            if k == 'tr_cst': op['itens'][0]['tributacaoRegular']['cst'] = v
            elif k == 'tr_cct': op['itens'][0]['tributacaoRegular']['cClassTrib'] = v
            elif k == 'tpDoc': op['tpDoc'] = v
            else: op['itens'][0][k] = v
    return f
N2, N3, N4, N7, I27, I26 = ('tpdoc_nfse_200043.json', 'tpdoc_nfcom_200044.json', 'tpdoc_sem_anexo_000001.json',
                           'tpdoc_tributacao_regular.json', 'tpdoc_imposto_seletivo_2027.json', 'tpdoc_imposto_seletivo_superfluo_2026.json')
caso('tpdoc2/ncmValidoDoAnexo', N2, item(nbs=None, ncm='87100000'))
caso('tpdoc2/nenhumaNomenclatura', N2, item(nbs=None))
caso('tpdoc2/nbsForaDoAnexo', N2, item(nbs='109052100'))
caso('tpdoc3/ncmValido', N3, item(ncm='87100000'))
caso('tpdoc3/ncmInvalido', N3, item(ncm='24021000'))
caso('tpdoc3/nbsValida', N3, item(nbs='115012000'))
caso('tpdoc3/nbsInvalida', N3, item(nbs='109052100'))
caso('tpdoc4/semNomenclatura', N4, item(ncm=None))
caso('tpdoc4/nbsEmNfe', N4, item(ncm=None, nbs='109052100'))
caso('tpdoc4/nfseNbs', N4, item(tpDoc=91, ncm=None, nbs='109052100'))
caso('tpdoc4/semTpDoc', N4, item(tpDoc=None))
caso('tpdoc4/semTpDocSemNomenclatura', N4, item(tpDoc=None, ncm=None))
caso('tpdoc7/semVinculo', N7, item(tr_cct='200044'))
caso('tpdoc7/ncmForaDoAnexo', N7, item(ncm='24021000'))
caso('tpdoc7/naoExigidaSemVinculo', N7, item(cst='000', cClassTrib='000001', ncm='24021000', tr_cst='200', tr_cct='200044'))
caso('tpdoc8/is2027', I27)
caso('tpdoc8/nfce', I27, item(tpDoc=65))
caso('tpdoc8/tpDoc62', I27, item(tpDoc=62))
caso('tpdoc8/superfluo2026', I26)
caso('tpdoc8/semTpDoc', I27, item(tpDoc=None))
json.dump(casos, open('casos_servicos.json', 'w'), ensure_ascii=False, indent=0)
print(len(casos))
