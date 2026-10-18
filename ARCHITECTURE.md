# DictOS Desktop — Arquitetura

## Princípios

1. **Núcleo independente de plataforma e de GUI.** `core` e `dictionary` não importam nada de sistema operacional nem de interface; são testáveis em CI puro.
2. **Integração de SO isolada.** Tudo que toca Win32/UIA/Wayland/Hyprland vive em `platform-*`. O núcleo conversa com traits.
3. **Popup atrás de trait.** A janela de consulta é uma interface; cada plataforma tem sua implementação.
4. **Dependências mínimas.** Cada crate novo precisa de justificativa real.

## Módulos

```
┌──────────────────────────────┐
│ dictos (binário: CLI/daemon) │
└───────┬──────────────┬───────┘
        │              │
   ┌────▼────┐   ┌─────▼──────┐
   │  popup  │   │  platform  │  ← traits + impls por SO
   │ (trait) │   │ (selection,│
   └────┬────┘   │  hotkey,   │
        │        │  clipboard)│
        │        └─────┬──────┘
        └──────┐ ┌─────┘
            ┌──▼─▼─────────┐
            │     core     │  ← modelos, normalização, consulta
            └──┬───────┬───┘
        ┌──────▼─┐  ┌──▼───────┐
        │dictionary│  │ settings │  ← fst+bincode │ SQLite (etapas 5-6)
        └────────┘  │ history  │
                    │ online   │  ← desligado por padrão
                    └──────────┘
```

## Árvore alvo do repositório

```
crates/
├── core/              # modelos, normalização, lógica de consulta (sem IO de SO)
├── dictionary/        # formato próprio, builder, reader, importadores
├── selection/         # trait de captura + normalização de texto
├── platform-windows/  # (Etapa 3) UIA, hotkey global, clipboard
├── platform-linux/    # (Etapa 7) ext-data-control, Hyprland IPC
├── popup/             # trait de UI + impls popup-slint / popup-gtk
├── online/            # (Etapa 6) provedores remotos + cache — off por padrão
├── settings/          # (Etapa 5) preferências (SQLite)
├── history/           # (Etapa 5) histórico opt-in (SQLite)
└── dictos/            # binário: CLI agora, daemon nas etapas 3+
data/
└── fixtures/          # dados de teste (datasets grandes ficam fora do git)
```

## Decisões técnicas

### Formato de dicionário v1 (implementado)

Dois arquivos por dicionário, num diretório:

- `index.fst` — mapa FST (crate `fst`) de `headword normalizado` → `offset u64` em `entries.bin`. Lookup exato em O(tamanho da chave); prefixo via autômato. Chaves ordenadas, duplicatas mescladas no build.
- `entries.bin` — registros length-prefixed: `[u32 LE tamanho][payload bincode do DictionaryEntry]`, lidos por seek+read a partir do offset.

Justificativa: dados imutáveis por release, lookup por headword é o caso dominante, FST é compacto e rápido. SQLite FTS5 só se um dia houver busca no texto das definições. StarDict/dictd rejeitados como formato interno (formatos de intercâmbio; usamos JSONL para intercâmbio de importação).

### Windows (Etapa 3-4)

- **Hotkey global:** crate `global-hotkey` (RegisterHotKey; funciona sem janela visível).
- **Captura:** crate `uiautomation` → `GetFocusedElement` → `TextPattern` → `GetSelection`. Posição do popup via `TextRange.GetBoundingRectangles()`. Processo deve ser DPI-aware.
- **Fallback clipboard:** `SendInput(Ctrl+C)` com espera por mudança de sequence number; backup e restauração de **todos** os formatos via `clipboard-win`, pulando formatos sintetizados/privados/owner-display. **Se a restauração falhar, a consulta falha e o usuário é avisado — nunca sobrescrever o clipboard silenciosamente.** Blocklist para apps que reagem a Ctrl+C (Excel, terminais).
- **Limites aceitos:** apps elevados (UIPI) e jogos/canvas ficam sem captura no MVP.

### Linux / Hyprland (Etapa 7)

- **Seleção:** protocolo `ext-data-control-v1` (Hyprland 0.52.1+), primary selection — sem roubar foco, cobre apps Wayland e XWayland.
- **Atalho:** bind no `hyprland.conf` (`bind = SUPER, D, exec, dictos --query`) acordando daemon single-instance; portal GlobalShortcuts (`ashpd`) como fallback portável.
- **Overlay:** GTK4 + `gtk4-layer-shell` (Slint/winit não suportam layer-shell). Posicionamento por âncora top-left + margens dinâmicas (layer-shell não tem coordenadas arbitrárias).
- **Gap aceito:** posição exata da seleção não é obtível no Wayland — popup perto do cursor (`hyprctl cursorpos`).

### Interface

- **Windows:** Slint (licença royalty-free 2.0, atribuição no About). Janela `no-frame` + `always-on-top`, posicionamento absoluto, hide/show. Perda de foco detectada via workaround Win32 (não há callback público no Slint).
- **Wayland:** GTK4 + gtk4-layer-shell (ver acima). O núcleo compartilhado torna a troca de toolkit barata.

### Dados e licenças

- **en–en empacotado:** WordNet 3.1 (licença permissiva; manter aviso de copyright no bundle).
- **en–pt e cobertura ampliada:** Kaikki.org/Wiktionary (CC BY-SA 3.0) como download opcional, com atribuição; dataset derivado relicenciado CC BY-SA.
- **Sem dados GPL** (FreeDict, GCIDE) — incompatíveis com a fusão com CC BY-SA.

### Privacidade

- `online` desligado por padrão; chamadas remotas só por ação explícita e informada.
- `history` opt-in, local.
- Cache de consultas online em SQLite local.

## Fluxo de consulta

```
hotkey → captura de seleção (platform-*) → normalize (core)
       → lookup offline (dictionary) → popup
       → [opcional, explícito] consulta online (online) → cache
```
