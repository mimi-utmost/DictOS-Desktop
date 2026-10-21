# DictOS Desktop — Instruções para agentes

## Início de sessão

- No início de toda sessão, verificar se o ai-memory tem versão nova; se tiver, atualizar pelo método original de instalação e avisar o usuário para reiniciar a sessão (a nova versão do MCP só ativa em sessão nova — o agente não reinicia a própria sessão).

## Fluxo de branches

- Nenhuma alteração vai direto para `master`.
- Todo trabalho acontece na `develop` ou em branches `feature/...` criadas a partir dela.
- Uma alteração só chega à `master` (versão estável) depois de:
  1. Estar integrada na `develop`;
  2. Passar pelos testes;
  3. Receber aprovação explícita do dono do projeto.

## Commits

- Mensagens no estilo *conventional commits* (`feat:`, `fix:`, `chore:`, `docs:`, ...).
- Commits pequenos e focados; descrição do "porquê" no corpo quando não for óbvio.

## Testes

- Não considerar uma tarefa concluída enquanto houver teste falhando.
- Rodar o build e os testes do projeto antes de abrir merge para `master`.
- Verificação padrão: `cargo test --workspace` verde.

## Arquitetura

- O núcleo (`crates/core`, `crates/dictionary`) não depende de sistema operacional nem de GUI e é testável em isolado.
- Integração com o sistema operacional só em crates `platform-*`; o núcleo conversa com traits.
- O popup é um trait com implementações por plataforma (Slint no Windows, GTK4+layer-shell no Wayland).
- Evitar dependências desnecessárias; cada crate nova precisa de justificativa real.
- Não implementar funcionalidade simulada e apresentá-la como concluída.
- Detalhes e decisões em `ARCHITECTURE.md`; requisitos em `SPEC.md`; etapas em `ROADMAP.md`.

## Dicionários e licenças

- Verificar a licença de qualquer dado antes de distribuí-lo; registrar atribuições.
- Dados empacotados: somente licenças permissivas (WordNet). Nada GPL nos dados.
- Dados CC BY-SA (Kaikki/Wiktionary) apenas como download opcional, com atribuição e share-alike nos dados derivados.

## Privacidade

- Nenhuma seleção do usuário é transmitida pela rede sem ação explícita e informada.
- Provedores online desligados por padrão; histórico opt-in e somente local.
- Nunca sobrescrever a área de transferência silenciosamente: se a restauração falhar, a consulta falha e o usuário é avisado.

## Rotina de limpeza (cleanUp)

Objetivo: não acumular metadados e artefatos gerados por IA ou pelo toolchain no armazenamento. Rodar ao fim de uma etapa/sessão ou quando o usuário pedir "cleanUp". **Nada nesta rotina pode quebrar o projeto**: só se apaga o que é regenerável ou temporário.

Checklist:
1. `cargo clean` — remove `target/` (artefatos de build, regeneráveis). Avisar que o próximo build fica lento.
2. Remover arquivos temporários criados por agentes: screenshots de teste (ex.: `%TEMP%\dictos-e2e.png`), scripts scratch fora de `scripts/`, logs soltos fora do repo.
3. Verificar `git status`: não deve restar nada não rastreado que seja artefato (build, log, temp). Artefatos recorrentes entram no `.gitignore`, nunca no git.
4. Revisar arquivos criados na sessão: documentação ou script temporário que cumpriu seu papel deve ser removido, não acumulado.

Nunca apagar nesta rotina:
- `data/` (dicionários gerados — regeneráveis, mas custosos), a menos que o usuário peça;
- `Cargo.lock` (obrigatório para builds reproduzíveis de binário);
- `.git/` e o histórico;
- dados do ai-memory (wiki/observações) — a limpeza deles é feita pelas ferramentas próprias do ai-memory, não por `rm`.

## Segurança

Estas regras cobrem também erros típicos de código gerado por IA. Valem para todo commit, sem exceção.

- **Nenhum segredo no git, nunca**: chaves de API, tokens, senhas, `.env`, chaves privadas e certificados ficam fora do repositório (`.gitignore`) e fora do código. Antes de cada commit, o agente deve revisar o diff procurando segredos hardcoded — inclusive os que ele mesmo gerou.
- **Segredo vazado = incidente**: se uma chave for parar no histórico, remover num commit posterior NÃO resolve. Rotacionar/revogar a credencial imediatamente e reescrever o histórico (ex.: `git filter-repo`). Avisar o usuário na hora.
- **Dados sensíveis em repouso são criptografados**: qualquer dado do usuário persistido localmente que seja sensível (seleções, histórico, cache de consultas) exige criptografia em repouso (ex.: DPAPI no Windows) — nunca texto puro.
- **Configuração com segredos**: apps que precisarem de chave (ex.: provedor online futuro) leem de variável de ambiente ou cofre do SO; nunca de arquivo commitado. Fornecer `.env.example` sem valores reais.
- **Desconfiar de entrada externa**: conteúdo capturado de seleção, respostas de rede e dados de dicionário importados são entrada não confiável — validar/limitar tamanho antes de processar ou exibir.
- **Dependências**: antes de adicionar uma crate, verificar manutenção e procedência; rodar `cargo audit` quando CI existir (Etapa 8).
- **Antivírus**: nunca implementar técnicas de evasão de AV (ofuscação, packing, strings criptografadas para esconder comportamento). Confiança se constrói com assinatura de código, metadados e transparência — ver "Confiança e antivírus" em SPEC.md.
