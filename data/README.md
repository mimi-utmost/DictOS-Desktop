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

## Gerar o dicionário en–pt (Kaikki/Wiktionary, opcional, CC BY-SA)

Traduções extraídas do Wiktionary em inglês por wiktextract, distribuídas por
kaikki.org. **Licença CC BY-SA 3.0**: nunca empacotado no aplicativo; o usuário
baixa e gera localmente. O import grava um `NOTICES` de atribuição dentro do
diretório do dicionário gerado.

```sh
# 1. Baixar o JSONL raw do enwiktionary (~523 MB .gz / ~3,3 GB puro)
curl -LO https://kaikki.org/dictionary/English/kaikki.org-dictionary-English.jsonl.gz
mv kaikki.org-dictionary-English.jsonl.gz data/raw/

# 2. Importar só as traduções pt (streaming; aceita .jsonl ou .jsonl.gz)
cargo run -p dictos -- import-kaikki data/raw/kaikki.org-dictionary-English.jsonl.gz --out data/dict/en-pt-kaikki

# 3. Consultar combinando com o en–en (definições do WordNet + traduções pt)
cargo run -p dictos -- lookup apple --dict data/dict/en-wordnet --dict data/dict/en-pt-kaikki
```

O dicionário en–pt contém **somente traduções** (as definições en–en vêm do
WordNet). Outro idioma: `--lang es`, `--lang de`, etc. (código ISO 639-1).
Para um teste rápido sem baixar tudo, dá para importar um prefixo do arquivo
puro (`curl -r 0-209715199 .../kaikki.org-dictionary-English.jsonl`) — foi o
método usado no E2E da Etapa 6 (605 entradas com tradução pt).

## Licenças

Ver `NOTICES.md` na raiz. Dados empacotados precisam ter licença compatível
(permissiva ou CC BY-SA com atribuição); dados GPL são proibidos no bundle.
