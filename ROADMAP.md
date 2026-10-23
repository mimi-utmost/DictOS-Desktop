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

## Etapa 6.1 — Correções críticas (bloqueadores de confiança)

Origem: auditoria técnica do teste de personas simuladas. Estas correções valem mais que feature nova — uma delas (clipboard) torna falsa a promessa central do app.

**Entrega:** restore do clipboard garantido em todo caminho de saída do fallback (inclusive falha de leitura e corrida pós-timeout); timeout e limite de tamanho explícitos no fetch online; botão "consultar online" desabilitado em telas de erro; aviso de privacidade legível e correto ("o texto selecionado" + serviço de destino); polling de `GetAsyncKeyState` restrito ao popup visível; índice do cache online com salt (HMAC local).

**Critérios de aceitação:**
- Teste prova que qualquer falha após o Ctrl+C restaura o clipboard; caminho de corrida documentado e mitigado.
- Fetch travado não congela o daemon (teste com servidor lento/sem resposta).
- Nenhum botão de rede disponível em tela de erro; consentimento legível (sem fonte minúscula).
- Polling de teclas parado quando o popup está escondido.
- `cargo test --workspace` verde.

## Etapa 6.2 — Dados e qualidade da consulta

Origem: convergência das 3 personas — sem tradução real e sem fallback, o app perde para o Google Tradutor.

**Entrega:** dataset en–pt completo (Kaikki full, ~523 MB .gz) com métrica de cobertura no import; fallback frase→palavra-base ("stumbled upon" → "stumble", com aviso "não achei a frase, mas aqui está a palavra base"); popup com tradução em destaque no topo, sentidos limitados aos mais comuns com "ver mais"; aviso explícito quando não há tradução, oferecendo a consulta online.

**Critérios de aceitação:**
- Palavras das personas ("warranty", "recall", "deadline", "serendipity", "obscure", "ephemeral") têm tradução pt.
- "stumbled upon" e "lasting impression" retornam algo útil (palavra base), não erro seco.
- Popup mostra tradução primeiro; sentidos comuns limitados com expansão.

## Etapa 6.3 — Tradução online de frases

**Entrega:** provedor online de tradução en→pt para texto selecionado (frases/parágrafos curtos), como ação explícita do usuário; candidatos sem chave (MyMemory, LibreTranslate) avaliados quanto a termos de uso e limites, com a recomendação registrada; seleção longa demais para o offline passa a oferecer tradução online em vez de erro; cache criptografado cobre frases; falha/limite do provedor vira mensagem clara.

**Critérios de aceitação:**
- Frase real traduzida en→pt via rede e exibida no popup com nota de origem.
- Reconsulta da mesma frase responde do cache sem rede.
- Nenhum envio sem clique explícito; consentimento nomeia "texto selecionado" e o serviço.
- Seleção longa → popup oferece a tradução online (nunca envia sozinho).

## Etapa 6.4 — UX para não-técnicos

**Entrega:** revisão de todos os textos visíveis ao usuário em linguagem humana ("resultado da internet" em vez de "dictionaryapi.dev"), mensagens de erro com próxima ação sugerida, e demais ajustes das personas que não envolvam distribuição.

**Critérios de aceitação:**
- Nenhum texto visível usa jargão técnico ou nome de API.
- Toda mensagem de erro sugere o que fazer em seguida.

## Etapa 7 — Distribuição Windows

**Entrega:** instalador Windows (clique-duplo, sem terminal — inclui baixar/gerar os dicionários), metadados completos do binário, assinatura Authenticode + timestamp, scan VirusTotal pré-release, tela About com todas as atribuições, CI com testes no Windows.

**Critérios de aceitação:**
- Instalador instala e executa o fluxo E2E numa máquina limpa.
- Binário release com metadados completos e assinado com Authenticode + timestamp.
- Scan do artefato final no VirusTotal antes de cada release; falso positivo é bloqueador: submeter ao fornecedor antes de publicar.
- CI verde para o workspace inteiro no Windows.

## Etapa 8 — Linux / Hyprland (adiada até o Windows estar estável e distribuível)

**Entrega:** `platform-linux` (ext-data-control para primary selection, bind no hyprland.conf + daemon single-instance) + `popup-gtk` (GTK4 + gtk4-layer-shell) posicionado pelo cursor (`hyprctl cursorpos`); instruções de build para Arch; CI do núcleo no Linux.

**Critérios de aceitação:**
- E2E no Hyprland equivalente ao Windows: selecionar → atalho → popup.
- Popup renderiza acima de outras janelas na camada overlay.
- Build no Arch documentado e reproduzido; CI verde no Linux para o núcleo.
