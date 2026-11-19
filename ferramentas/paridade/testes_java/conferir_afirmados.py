"""confere nas respostas gravadas do Java os valores que os testes unitários
afirmavam (esperados_*.json do unitarios.py/unificada.py): garante que a
conversão para HTTP reproduz o mesmo cenário

uso: python3 conferir_afirmados.py esperados_X.json golden_X.json"""
import json, sys
from decimal import Decimal


def pares(v):
    # unitarios.py guarda só o valor de baseCalculo
    return [('baseCalculo', v)] if isinstance(v, str) else v


def main():
    esperados = json.load(open(sys.argv[1]))
    gravadas = {g['id']: g for g in json.load(open(sys.argv[2]))}
    divergentes = 0
    for cid, v in esperados.items():
        g = gravadas[cid]
        corpo = json.loads(g['body']) if g['status'] == 200 else {}
        for campo, valor in pares(v):
            if campo not in corpo or Decimal(str(corpo[campo])) != Decimal(valor):
                divergentes += 1
                print('diverge', cid, campo, corpo.get(campo), 'esperado', valor)
    print(len(esperados), 'casos conferidos,', divergentes, 'divergentes')
    sys.exit(1 if divergentes else 0)


if __name__ == '__main__':
    main()
