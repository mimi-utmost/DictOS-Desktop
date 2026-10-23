# DictOS Desktop — Especificação

## Visão

Aplicativo desktop inspirado na experiência de consulta rápida do Apple Dictionary (macOS), **sem copiar sua identidade visual**: o usuário seleciona texto em qualquer aplicativo compatível, aciona um atalho global e vê a definição numa janela flutuante compacta e discreta, sem interromper a leitura ou o trabalho.

- **Primeiro sistema-alvo:** Windows.
- **Segundo sistema-alvo:** Linux, especialmente Arch Linux com Hyprland (Wayland).
- **Linguagem:** Rust.
- **Dicionários:** offline por padrão, com consulta online opcional.

## Requisitos funcionais

1. Consultar palavras selecionadas em outros aplicativos.
2. Registrar um atalho global configurável.
3. Mostrar definição, classe gramatical, tradução e exemplos quando disponíveis.
4. Suportar dicionários inglês–inglês e inglês–português.
5. Funcionar offline.
6. Permitir provedores online opcionais e cache — incluindo **tradução online de frases e parágrafos curtos (en→pt)** como complemento do dicionário offline, que cobre palavras.
7. Preservar a área de transferência sempre que possível.
8. Tratar falhas de captura, palavras desconhecidas e indisponibilidade da internet.
9. Permitir configuração de idiomas, fontes e comportamento da janela.
10. Respeitar a privacidade (ver seção própria).
11. Quando a seleção for longa demais para o dicionário offline, oferecer a consulta online em vez de um erro sem saída.

## Privacidade

- Provedores online ficam **desligados por padrão**.
- **Nenhuma seleção é transmitida pela rede sem uma ação claramente informada do usuário.** O aviso deve ser legível e nomear o que sai da máquina ("o texto selecionado", não apenas "a palavra") e o serviço de destino em linguagem humana. Frases podem conter conteúdo pessoal — o consentimento precisa refletir isso.
- Histórico de consultas é **opt-in** e fica apenas na máquina do usuário, com as palavras criptografadas em repouso (DPAPI no Windows; keyring no Linux, Etapa 8).
- O cache de consultas online é local, com corpo criptografado e índice por hash com salt (Etapa 6.1).
- Criptografia DPAPI protege contra outros usuários da máquina e contra disco roubado; **não protege contra malware rodando como o mesmo usuário** — esta limitação é declarada, não escondida.

## Requisitos não funcionais

- Núcleo (modelos, dicionário, lógica de consulta) **testável sem interface gráfica** e sem dependência de sistema operacional.
- Integração com cada sistema isolada em crates próprios (`platform-windows`, `platform-linux`).
- Evitar dependências desnecessárias.
- Não implementar funcionalidade simulada e apresentá-la como concluída.

## Confiança e antivírus

O app tem, por natureza, comportamentos que antivírus associam a keyloggers/spyware: hotkey global, leitura de seleção de outros apps (UIA), teclas sintéticas no fallback, polling de `GetAsyncKeyState`. A estratégia é **transparência e reputação, nunca evasão** — técnicas para "despistar" antivírus (ofuscação, packing, criptografia de strings) são prática de malware, aumentam falsos positivos e estão proibidas neste projeto.

Medidas obrigatórias para releases:

- **Assinatura de código** (Authenticode + timestamp; certificado EV dá reputação SmartScreen imediata, OV acumula com o tempo).
- **Metadados completos** no binário: manifesto com identidade de assembly, CompanyName, FileDescription, versão — binário sem metadados parece mais suspeito para heurísticas.
- **Sem packing/ofuscação**: binário release compilado direto, sem UPX nem similares.
- **Scan pré-release no VirusTotal**; qualquer detecção é bloqueador e vira submissão de falso positivo ao fornecedor antes da publicação.
- **Código aberto + build reproduzível**: quem desconfia pode auditar e compilar — é a base da confiança, junto com a assinatura que liga o binário ao fonte.
- **Comportamento mínimo e documentado**: a captura de texto só acontece no momento do atalho — nenhum texto é lido continuamente. Hoje existe um polling contínuo de `GetAsyncKeyState` (somente Esc e botão esquerdo) para detectar fechamento do popup; a Etapa 6.1 restringe esse polling aos momentos em que o popup está visível, para alinhar a prática a esta promessa.
- Ferramentas de desenvolvimento que disparam AV (ex.: `scripts/e2e-popup.ps1`, que faz screenshot + teclas sintéticas) são documentadas como tal; exclusões de antivírus em ambiente de dev são configuração do usuário, nunca algo que o app ou os scripts configurem sozinhos.

## Licenciamento

- **Código:** dual MIT / Apache-2.0.
- **Interface:** Slint sob licença royalty-free 2.0 — exige atribuição visível ("Built with Slint" no About).
- **Dados empacotados:** WordNet 3.1 (licença permissiva estilo BSD, exige manter aviso de copyright).
- **Dados opcionais (download):** Kaikki.org / Wiktionary sob CC BY-SA 3.0 — exige atribuição e share-alike **nos dados**, não no código.
- **Proibido no bundle:** dados sob GPL (FreeDict, GCIDE) — incompatíveis com a fusão com dados CC BY-SA e desnecessários dado WordNet + Kaikki.
- Nenhum dicionário é distribuído sem verificação de licença.

## Riscos e decisões em aberto

| Risco | Mitigação |
|---|---|
| Chromium/Electron não expõem seleção via UIA por padrão; provider UIA do Chrome ~140+ é instável | Fallback de clipboard; testar contra versões correntes na Etapa 3 |
| Processo em background não toma foreground no Windows: popup abre sem foco | Resolvido na Etapa 4: popup NOACTIVATE por design, fecha por Esc/×/clique fora via polling global; foco permanece no app de origem |
| Apps elevados bloqueiam UIA e SendInput (UIPI) | Fora do MVP; exigiria build assinado com `uiAccess` — documentar |
| Apps que reagem mal a Ctrl+C simulado (Excel, terminais) | Blocklist de classes de janela; nunca sobrescrever o clipboard silenciosamente |
| Cobertura de traduções en–pt do Kaikki é irregular | Dataset completo gerado na Etapa 6.2; métrica de cobertura no import |
| Wayland não expõe posição da seleção de outros apps | Popup posicionado perto do cursor do mouse (`hyprctl cursorpos`) |
| GNOME/Wayland não implementa ext-data-control | Fora do alvo; afeta só portabilidade futura |
