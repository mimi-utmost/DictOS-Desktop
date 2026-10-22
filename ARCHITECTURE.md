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
├── dictionary/        # formato próprio, builder, reader (lookup_merged multi-dict),
│                      # importadores (JSONL, WordNet, Kaikki)
├── selection/         # trait de captura + tipos + extract_query (sem SO)
├── platform-windows/  # UIA (uiautomation), hotkey global (global-hotkey), fallback
│                      # de clipboard com backup/restore (crate windows)
├── platform-linux/    # (Etapa 7) ext-data-control, Hyprland IPC
├── popup/             # trait Popup + LookupView + implementação Slint;
│                      # popup-gtk (Wayland) na Etapa 7
├── online/            # provedor dictionaryapi.dev (ureq) + cache criptografado —
│                      # só dispara por ação explícita do usuário
├── settings/          # preferências (SQLite)
├── history/           # histórico opt-in (SQLite)
└── dictos/            # binário: CLI (import/lookup/suggest) + daemon (run)
data/
└── fixtures/          # dados de teste (datasets grandes ficam fora do git)
```

## Decisões técnicas

### Formato de dicionário v1 (implementado)

Dois arquivos por dicionário, num diretório:

- `index.fst` — mapa FST (crate `fst`) de `headword normalizado` → `offset u64` em `entries.bin`. Lookup exato em O(tamanho da chave); prefixo via autômato. Chaves ordenadas, duplicatas mescladas no build.
- `entries.bin` — registros length-prefixed: `[u32 LE tamanho][payload bincode do DictionaryEntry]`, lidos por seek+read a partir do offset.

Justificativa: dados imutáveis por release, lookup por headword é o caso dominante, FST é compacto e rápido. SQLite FTS5 só se um dia houver busca no texto das definições. StarDict/dictd rejeitados como formato interno (formatos de intercâmbio; usamos JSONL para intercâmbio de importação).

### Windows (implementado nas Etapas 3 e 4)

- **Hotkey global:** crate `global-hotkey` (RegisterHotKey; funciona sem janela visível). Padrão `Ctrl+Shift+D`, configurável via `--hotkey`.
- **Captura:** crate `uiautomation` → `GetFocusedElement` → `TextPattern` → `GetSelection`. Sem seleção exposta, cai para o fallback de clipboard **se a política do app permitir**.
- **Política por app (Etapa 4):** classe da janela em foreground decide: terminais (`ConsoleWindowClass`, `CASCADIA_HOSTING_WINDOW_CLASS`, `mintty`) e Excel (`XLMAIN`) nunca recebem Ctrl+C simulado (interrompe o shell / copia a célula); UIA ainda é tentada. Janela do próprio DictOS: captura ignorada.
- **Fallback clipboard:** `SendInput(Ctrl+C)` (com key-up prévio dos modificadores do hotkey) + espera por mudança de `GetClipboardSequenceNumber` + backup/restauração de todos os formatos (pulando sintetizados, owner-display, privados e delayed-rendered), implementado sobre a crate `windows`. **Se a restauração falhar, a consulta é abortada e o usuário avisado — nunca sobrescrever o clipboard silenciosamente.** Coberto por teste automatizado real.
- **Âncora (Etapa 4):** `IUIAutomationTextRange::GetBoundingRectangles` via `AsRef` (o crate `uiautomation` resolve para o mesmo `windows` 0.62) → primeiro retângulo não degenerado do SAFEARRAY. Sem retângulo (seleção rolada para fora etc.), cai no cursor do mouse.
- **Posição do popup (Etapa 4):** abaixo da âncora (acima se não couber), clampado à work area do monitor da âncora (`MonitorFromRect` + `GetMonitorInfoW`). Posicionamento em pixels físicos (`PhysicalPosition`).
- **DPI (Etapa 4):** processo Per-Monitor V2 (`SetProcessDpiAwarenessContext` no início do daemon) — todas as coordenadas (UIA, cursor, work area, popup) são físicas e consistentes entre monitores de escalas diferentes.
- **Foco (Etapa 4):** o popup é `WS_EX_NOACTIVATE` + `SWP_NOACTIVATE` e restaura o foreground anterior se o `show()` do winit ativar a janela mesmo assim. Como nunca tem foco, Esc e clique-fora são detectados por polling global de `GetAsyncKeyState` (bit "pressionada desde a última chamada", que não perde apertos curtos). × continua no popup.
- **Limites aceitos:** apps elevados (UIPI) e jogos/canvas ficam sem captura no MVP.

### Persistência (implementado na Etapa 5)
- **Banco único SQLite** (`rusqlite` bundled) em `%APPDATA%\DictOS\dictos.db`, override via `DICTOS_DATA_DIR`. Schema versionado por `PRAGMA user_version`.
- **`settings`**: chave-valor (`hotkey`, `history.enabled`, `popup.font_scale`). O daemon lê na inicialização; a flag `--hotkey` tem precedência sobre o valor persistido.
- **`history`**: opt-in (desligado por padrão), somente local. A palavra consultada é dado sensível → **sempre criptografada antes de tocar o disco** via trait `Protector`; no Windows, `DpapiProtector` (CryptProtectData, escopo do usuário, sem gerenciamento de chaves). Falha de proteção aborta a gravação — nunca grava em texto puro. Armazena palavra cifrada + achou/não achou + timestamp.

### Online e cache (implementado na Etapa 6)

- **Multi-dicionário:** `--dict` é repetível no CLI e no daemon; `lookup_merged` consulta os dicionários em ordem e funde acepções/traduções num único resultado (en–en do WordNet + traduções pt do Kaikki no mesmo popup).
- **Kaikki/Wiktionary (CC BY-SA):** importador streaming (`import_kaikki_translations`) extrai só traduções do idioma-alvo do JSONL raw (aceita `.jsonl` e `.jsonl.gz`); gera entradas só-tradução e grava `NOTICES` de atribuição no diretório do dicionário. Nunca empacotado.
- **Provedor online:** trait `OnlineProvider`; implementação `DictionaryApiDev` (dictionaryapi.dev, en–en, sem chave) via `ureq` (HTTP simples, sem runtime async). Parse da resposta é função pura testável.
- **Ação explícita obrigatória:** o único caminho de rede é a flag `--online` (CLI) ou o botão "🌐 Consultar online (envia a palavra)" no popup — o rótulo deixa claro que a palavra sai da máquina. O resultado é exibido com nota de origem ("via dictionaryapi.dev (online)").
- **Cache:** tabela `online_cache` no mesmo SQLite; chave = SHA-256 da palavra + provedor, corpo da resposta criptografado via `Protector` (DPAPI no Windows) — nem a palavra nem o conteúdo ficam em texto puro em repouso. Reconsulta funciona sem rede.

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
- Cache de consultas online em SQLite local, com corpo criptografado (DPAPI) e índice por hash da palavra.

## Fluxo de consulta

```
hotkey → captura de seleção (platform-*) → normalize (core)
       → lookup offline (dictionary) → popup
       → [opcional, explícito] consulta online (online) → cache
```
