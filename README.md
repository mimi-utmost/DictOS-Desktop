# DictOS Desktop

Dicionário popup offline-first inspirado na experiência de consulta rápida do
Apple Dictionary: selecione texto em qualquer aplicativo, pressione um atalho
global e veja a definição numa janela flutuante, sem interromper o trabalho.

- **Alvo 1:** Windows · **Alvo 2:** Arch Linux + Hyprland (Wayland)
- **Linguagem:** Rust · **Interface:** Slint (Windows) / GTK4+layer-shell (Wayland)
- **Dicionários:** offline por padrão (WordNet 3.1), online opcional, en–en e en–pt

Requisitos em [SPEC.md](SPEC.md) · Arquitetura em [ARCHITECTURE.md](ARCHITECTURE.md)
· Licenças de dados em [NOTICES.md](NOTICES.md)

## Status

**Etapa 5 de 8 concluída** — preferências persistidas em SQLite (`hotkey`,
`popup.font_scale`, `history.enabled`) e histórico opt-in com as palavras
consultadas **criptografadas em repouso via DPAPI** (nunca texto puro).
Novos comandos: `config get/set/list` e `history list/clear/on/off`.

## Roadmap

| Etapa | Entrega | Status |
|---|---|---|
| 1. Fundação | Workspace, `core` + `dictionary` (fst), CLI `import/lookup/suggest`, 13 testes | ✅ |
| 2. Dados reais | Importador WordNet 3.1, dataset en–en, NOTICES, pipeline reproduzível | ✅ |
| 3. MVP Windows | Hotkey global + captura UIA + fallback clipboard + popup Slint + daemon | ✅ |
| 4. Robustez Windows | Posicionamento junto à seleção, blocklist, DPI, erros de captura | ✅ |
| 5. Config + histórico | Preferências (SQLite), histórico opt-in criptografado (DPAPI), janela configurável | ✅ |
| 6. en–pt + online | Traduções Kaikki (CC BY-SA), provedor online opcional com cache | ⬜ próxima |
| 7. Linux/Hyprland | ext-data-control, bind no hyprland.conf, popup GTK4 layer-shell | ⬜ |
| 8. Distribuição | Instalador Windows, build Arch, CI, tela About | ⬜ |

Detalhes e critérios de aceitação de cada etapa em [ROADMAP.md](ROADMAP.md).

## Uso atual

```sh
# Gerar o dicionário offline (uma vez — ver data/README.md)
cargo run -p dictos -- import-wordnet data/raw/dict --out data/dict/en-wordnet

# Consultar via CLI
cargo run -p dictos -- lookup serendipity --dict data/dict/en-wordnet
cargo run -p dictos -- suggest serend --dict data/dict/en-wordnet

# Daemon: selecione texto em qualquer app e pressione Ctrl+Shift+D
cargo run -p dictos -- run --dict data/dict/en-wordnet
cargo run -p dictos -- run --dict data/dict/en-wordnet --hotkey "Ctrl+Alt+D"

# Preferências e histórico (opt-in, criptografado via DPAPI)
cargo run -p dictos -- config list
cargo run -p dictos -- config set popup.font_scale 1.2
cargo run -p dictos -- history on && cargo run -p dictos -- history list

# Teste E2E manual do popup (abre Bloco de Notas, seleciona, aciona o atalho)
powershell -ExecutionPolicy Bypass -File scripts/e2e-popup.ps1
```

## Desenvolvimento

```sh
cargo test --workspace   # 58 testes
```

## Branches

- `master` — versão estável (só recebe merges testados e aprovados)
- `develop` — branch de desenvolvimento; novas features saem daqui

## Licença

Código sob dual MIT/Apache-2.0 (a formalizar na Etapa 8). Dados de dicionário
sob suas próprias licenças — ver [NOTICES.md](NOTICES.md).
