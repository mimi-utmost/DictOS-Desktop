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
6. Permitir provedores online opcionais e cache.
7. Preservar a área de transferência sempre que possível.
8. Tratar falhas de captura, palavras desconhecidas e indisponibilidade da internet.
9. Permitir configuração de idiomas, fontes e comportamento da janela.
10. Respeitar a privacidade (ver seção própria).

## Privacidade

- Provedores online ficam **desligados por padrão**.
- **Nenhuma seleção é transmitida pela rede sem uma ação claramente informada do usuário** (ex.: botão "consultar online" ou toggle explícito nas configurações).
- Histórico de consultas é **opt-in** e fica apenas na máquina do usuário, com as palavras criptografadas em repouso (DPAPI no Windows; keyring no Linux, Etapa 7).
- O cache de consultas online é local.

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
- **Comportamento mínimo e documentado**: a captura só acontece no momento do atalho; nada é monitorado continuamente. Isso é requisito de design, não só de marketing — reduz a superfície que heurísticas flagam.
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
| Cobertura de traduções en–pt do Kaikki é irregular | Múltiplas fontes CC BY-SA; métrica de cobertura na Etapa 6 |
| Wayland não expõe posição da seleção de outros apps | Popup posicionado perto do cursor do mouse (`hyprctl cursorpos`) |
| GNOME/Wayland não implementa ext-data-control | Fora do alvo; afeta só portabilidade futura |
