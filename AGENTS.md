# DictOS Desktop — Instruções para agentes

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
