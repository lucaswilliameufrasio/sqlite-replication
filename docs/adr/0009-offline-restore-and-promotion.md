# ADR 0009: restore offline em temporário e promoção sem overwrite

- **Status:** Aceita para M1 (D-11)
- **Data:** 2026-10-23

## Contexto

Um restore incompleto ou concorrente com a aplicação pode substituir um banco
válido ou deixar um destino inconsistente.

## Decisão

Decodificar LTX para arquivo temporário no diretório de destino, validar o
arquivo e rodar `quick_check` e `integrity_check`. Promover apenas se o destino
final não existir, usando operação atômica que falha quando já existe.

## Alternativas rejeitadas

- Aplicar páginas no banco ativo: pode expor estado parcial.
- Substituir destino existente por padrão: risco destrutivo sem autorização.
- Restaurar diretamente para o caminho final: deixa arquivo parcial em falha.

## Consequências

`RestoreIfMissing` é o comportamento inicial. Substituição explícita e
durabilidade específica de plataforma são trabalho posterior e exigem
precondições documentadas.

## Reconsiderar quando

Somente ao definir um modo de substituição explicitamente solicitado, com
isolamento, rollback e testes de crash.
