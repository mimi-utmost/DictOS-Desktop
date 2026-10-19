# data/ — Pipeline de dados do dicionário

Os dados brutos e os dicionários gerados **não são versionados no git**
(`.gitignore` cobre `data/raw/` e `data/dict/`). Apenas fixtures pequenos de
teste ficam no repositório (`data/fixtures/`, `crates/*/tests/fixtures/`).

## Regenerar o dicionário en–en (WordNet 3.1)

```sh
# 1. Baixar o banco oficial (~16 MB)
curl -LO https://wordnetcode.princeton.edu/wn3.1.dict.tar.gz
tar -xzf wn3.1.dict.tar.gz -C data/raw/

# 2. Importar para o formato v1 do DictOS
cargo run -p dictos -- import-wordnet data/raw/dict --out data/dict/en-wordnet

# 3. Consultar
cargo run -p dictos -- lookup serendipity --dict data/dict/en-wordnet
```

Resultado atual: ~155 mil entradas, `index.fst` ≈ 1,5 MB, `entries.bin` ≈ 30 MB.

## Licenças

Ver `NOTICES.md` na raiz. Dados empacotados precisam ter licença compatível
(permissiva ou CC BY-SA com atribuição); dados GPL são proibidos no bundle.
