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
