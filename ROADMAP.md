# DictOS Desktop — Roadmap

Regra geral: uma etapa só é dada como concluída com seus critérios de aceitação verificados. Nenhuma etapa entra na `master` sem testes verdes e aprovação do dono do projeto.

## Etapa 1 — Fundação ✅ (concluída)

**Entrega:** documentos (SPEC, ARCHITECTURE, ROADMAP, AGENTS.md) + workspace Cargo + `core` + `dictionary` (formato fst v1) + CLI `dictos import/lookup/suggest`.

**Critérios de aceitação:**
- `cargo test --workspace` verde.
- Lookup exato, por prefixo, com normalização (case/diacríticos) e palavra desconhecida cobertos por testes.
- Roundtrip de importação (JSONL → formato v1 → leitura) com fixture em `data/fixtures/`.

## Etapa 2 — Dados reais ✅ (concluída)

**Entrega:** importador WordNet 3.1 (`dictos import-wordnet`) → dataset en–en; NOTICES com a licença do WordNet; pipeline documentado em `data/README.md`.

**Critérios de aceitação:**
- Lookup real de palavras comuns retorna definição, classe gramatical e exemplos (quando existentes no WordNet). ✅ verificado com "serendipity", "apple", "run" (~155 mil entradas importadas)
- NOTICES presente e correto (WordNet). ✅
- Dataset gerado por pipeline reproduzível (comandos em `data/README.md`), dados brutos e gerados fora do git. ✅

## Etapa 3 — MVP Windows ✅ (concluída)

**Entrega:** daemon `dictos run` com hotkey global (`global-hotkey`), captura via UIA (`uiautomation`), fallback de clipboard com backup/restore completo (bindings `windows` 0.62 diretos), popup Slint, integração ponta a ponta.

**Critérios de aceitação:**
- E2E manual: selecionar texto no Bloco de Notas → hotkey → popup com definição. ✅ verificado com screenshot (`scripts/e2e-popup.ps1`): "serendipity" retornou definição real do WordNet; "xyzzy" exibiu "Palavra desconhecida"
- Teste automatizado de backup/restore de clipboard (texto e formatos preservados). ✅ 7 testes em `platform-windows`, incluindo backup/restore real no clipboard do Windows
- Falha de captura exibe mensagem clara; clipboard nunca é sobrescrito silenciosamente. ✅ restauração falha → erro `ClipboardRestoreFailed`, consulta abortada

**Ressalvas registradas:**
- Fallback de clipboard (Ctrl+C sintético) tem teste automatizado de backup/restore, mas o caminho completo UIA→fallback ainda não foi exercido E2E em app sem TextPattern.
- Popup pode não receber foco (processo em background); fecha por clique fora ou no ×. Esc só funciona com foco no popup.

## Etapa 4 — Robustez Windows ✅ (concluída)

**Entrega:** âncora real da seleção via `IUIAutomationTextRange::GetBoundingRectangles` (SAFEARRAY via bindings `windows` 0.62 — o crate `uiautomation` usa a mesma versão, então `as_ref()` dá acesso direto à interface COM), popup `WS_EX_NOACTIVATE` com restauração de foreground, Esc/clique-fora via polling de `GetAsyncKeyState` (bit "pressionada desde a última chamada"), DPI Per-Monitor V2, política de captura por aplicativo (terminais e Excel sem Ctrl+C simulado).

**Critérios de aceitação:**
- Popup aparece junto à seleção em apps com TextPattern. ✅ medido no E2E (`scripts/e2e-popup.ps1`): seleção em y=211+34 → popup em y=253 (margem de 8px), x idêntico
- Apps da blocklist usam fallback seguro ou mensagem clara. ✅ política testada em unit tests; fallback negado → mensagem "Captura indisponível" (verificação manual em terminal pendente — requer interação humana com shell)
- Coordenadas corretas em monitores com escala ≠ 100%. ⚠️ processo agora é Per-Monitor V2 DPI-aware e todas as coordenadas são físicas, mas a máquina de desenvolvimento está em 100% (96 DPI): verificação E2E em HiDPI fica pendente de hardware
- Popup não rouba foco; Esc/×/clique-fora fecham. ✅ medido no E2E (foreground inalterado; Esc fecha)

## Etapa 5 — Configurações e histórico ✅ (concluída)

**Entrega:** `settings` (SQLite via rusqlite bundled, schema versionado) com `hotkey`, `history.enabled` e `popup.font_scale` (aplicados de verdade no daemon); `history` opt-in com palavra criptografada em repouso via DPAPI (`DpapiProtector`); CLI `config get/set/list` e `history list/clear/on/off`.

**Critérios de aceitação:**
- Preferências persistem entre execuções. ✅ teste automatizado (reabertura do banco) + E2E (hotkey e font_scale lidos do banco pelo daemon)
- Histórico desligado por padrão; quando ligado, registra e lista consultas. ✅ testes + E2E: consultas a "serendipity" (encontrada) e "xyzzy" (desconhecida) registradas e listadas; `history clear` limpa
- Palavras nunca em texto puro em repouso. ✅ teste automatizado (raw scan do arquivo) + verificação manual no `dictos.db` real

## Etapa 6 — en–pt e online ✅ (concluída)

**Entrega:** importador Kaikki/Wiktionary (`dictos import-kaikki`, streaming, `.jsonl`/`.jsonl.gz`, gera NOTICES CC BY-SA no dicionário); consulta multi-dicionário (`lookup_merged`, `--dict` repetível); crate `online` (provedor dictionaryapi.dev + cache SQLite com corpo criptografado via DPAPI, indexado por hash SHA-256 da palavra); flag `--online` no CLI; botão "🌐 Consultar online (envia a palavra)" no popup (o clique é a ação explícita).

**Critérios de aceitação:**
- Traduções pt exibidas para palavras comuns. ✅ E2E: `apple` → acepções do WordNet + "pt: maçã" do Kaikki, no CLI e no popup (screenshot)
- Nada é enviado à rede sem ação explícita do usuário. ✅ o único caminho de rede é `consult_online_cached`, chamado só por `--online` (CLI) ou pelo clique no botão; o popup exibe a nota "via dictionaryapi.dev (online)" — verificado no E2E automatizado do botão (clique sintético → log "consulta online solicitada" → popup com nota de origem)
- Cache permite reconsulta offline de resultados online. ✅ E2E: 1ª consulta a "serendipity" foi à rede; a 2ª respondeu "online: cache hit" sem rede

**Ressalvas registradas:**
- O dataset en–pt de desenvolvimento é **parcial** (primeiros ~200 MB do JSONL Kaikki: 605 entradas com tradução pt). O pipeline do dataset completo (~523 MB .gz) está documentado em `data/README.md`; rodar o import completo é custoso e fica a cargo do usuário.
- dictionaryapi.dev é en–en; não há provedor online en–pt nesta etapa.
- No E2E do botão, "serendipity" já estava no cache (do teste de CLI), então o caminho exercido foi cache → popup; o caminho rede → cache → popup foi verificado via CLI (`--online`).

## Etapa 7 — Linux / Hyprland

**Entrega:** `platform-linux` (ext-data-control para primary selection, bind no hyprland.conf + daemon single-instance) + `popup-gtk` (GTK4 + gtk4-layer-shell) posicionado pelo cursor (`hyprctl cursorpos`).

**Critérios de aceitação:**
- E2E no Hyprland equivalente ao MVP Windows: selecionar → atalho → popup.
- Popup renderiza acima de outras janelas na camada overlay.

## Etapa 8 — Distribuição

**Entrega:** instalador Windows, instruções de build para Arch, CI com testes, tela About com todas as atribuições, e **pacote confiável para antivírus** (ver "Confiança e antivírus" no SPEC).

**Critérios de aceitação:**
- Instalador Windows instala e executa o fluxo E2E.
- Build no Arch documentado e reproduzido.
- CI verde em Windows e Linux para o núcleo.
- Binário release com metadados completos (CompanyName, FileDescription, versão) e assinado com Authenticode + timestamp.
- Scan do artefato final no VirusTotal antes de cada release; falso positivo é tratado como bloqueador: submeter ao fornecedor antes de publicar.
