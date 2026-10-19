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
- Histórico de consultas é **opt-in** e fica apenas na máquina do usuário.
- O cache de consultas online é local.

## Requisitos não funcionais

- Núcleo (modelos, dicionário, lógica de consulta) **testável sem interface gráfica** e sem dependência de sistema operacional.
- Integração com cada sistema isolada em crates próprios (`platform-windows`, `platform-linux`).
- Evitar dependências desnecessárias.
- Não implementar funcionalidade simulada e apresentá-la como concluída.

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
| Apps elevados bloqueiam UIA e SendInput (UIPI) | Fora do MVP; exigiria build assinado com `uiAccess` — documentar |
| Apps que reagem mal a Ctrl+C simulado (Excel, terminais) | Blocklist de classes de janela; nunca sobrescrever o clipboard silenciosamente |
| Cobertura de traduções en–pt do Kaikki é irregular | Múltiplas fontes CC BY-SA; métrica de cobertura na Etapa 6 |
| Wayland não expõe posição da seleção de outros apps | Popup posicionado perto do cursor do mouse (`hyprctl cursorpos`) |
| GNOME/Wayland não implementa ext-data-control | Fora do alvo; afeta só portabilidade futura |
