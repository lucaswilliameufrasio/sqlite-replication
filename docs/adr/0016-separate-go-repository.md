# ADR 0016: separar a implementação Go

- **Status:** aceita para separação dos repositórios; revisão de licença e notices pendente
- **Data:** 2026-11-09

## Contexto e decisão

O módulo Go no subdiretório `go/` não podia ser instalado pelo caminho público
declarado. A pedido do mantenedor, a implementação foi extraída para
`github.com/lucaswilliameufrasio/sqlite-replication-go`, com `go.mod` na raiz.
Este repositório passa a manter a implementação Rust e os testes de
interoperabilidade. Esta decisão substitui o caminho Go do ADR 0014, sem
aprovar licença final ou publicação de pacote.

O CI fixa explicitamente o commit Go `cd9423694e4de94beeb97e023ca5fc1fe3f5ffee`;
localmente, `GO_REPO` aponta para um checkout independente. As fixtures são
copiadas sem regeneração para os testes próprios de cada implementação. A
conformance cruzada continua comparando as implementações e o oráculo
independente; os históricos de evidências anteriores permanecem neste
repositório. Atualizações do pin exigem executar o gate novamente.

Uma alternativa seria depender de `main` remoto em cada execução, mas isso
tornaria resultados históricos não reproduzíveis. O checkout separado implica
manter os dois repositórios e sincronizar intencionalmente o pin e as fixtures.
Os testes de S3 reais e drills opt-in continuam fora do gate normal.
