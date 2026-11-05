# ADR 0013: clientes S3 nativos por linguagem

- **Status:** Aceita para M3
- **Data:** 2026-10-26

## Contexto

M3 precisa manter adapters Go/Rust independentes, operar contra endpoint S3
configurável (inclusive Floci), suportar uploads grandes, precondições, retries
e cancelamento sem credenciais reais nos testes. Na decisão original, o
workspace declarava `rust-version = "1.85"` e a MSRV não poderia ser elevada
sem validação explícita.

## Decisão

- Go usa o SDK oficial AWS SDK for Go v2 (`config`, `credentials` e `service/s3`),
  com versões fixadas no `go.mod`/`go.sum` e configuração explícita do endpoint.
- Rust usa o AWS SDK oficial `aws-sdk-s3` fixado inicialmente em `1.90.0`, cuja
  versão declara MSRV 1.82 e fornece controle explícito de `If-None-Match` também
  no `CompleteMultipartUpload`. A versão ficou exata até que uma atualização
  passe a validação da MSRV e da conformance.
- Cada adapter continuará a expor o contrato próprio da linguagem e a declarar
  limitações em `Capabilities`; não se assume que Floci prova todo provedor.
- Retry é responsabilidade da política configurada no SDK, não de loops
  aninhados nos adapters. Go usa o retryer padrão do SDK; clientes Rust devem
  definir `RetryConfig::standard()` com limite explícito de tentativas e timeout
  de operação. Cancelamento/deadline do chamador limita a operação inteira.
- Testes de integração usam somente Floci fixado pelo ADR 0008, credenciais
  sintéticas, projeto/volume isolados e endpoint local.

## Consequências

Dependências transitivas e versões devem ser travadas e verificadas pelo gate
normal de cada linguagem. Atualizações dos SDKs devem validar explicitamente a
MSRV e repetir a conformance em Floci. A escolha não habilita WAL incremental
nem altera o gate CP-01.

## Atualização de runtimes e dependências — 2026-11-05

Após a solicitação do mantenedor para atualizar tecnologias e dependências antes
de continuar os marcos, a linha de base foi elevada para Go 1.27.3, Rust/Cargo
1.99.1 e AWS SDK for Rust S3 1.157.0. O SDK S3 atual exige Rust 1.94.1; portanto
o `rust-version` público do workspace também passa de 1.85 para 1.94.1. Essa
redução do conjunto de compiladores suportados é intencional e deve aparecer nas
notas de migração. `make test` passou após a atualização, incluindo a
conformance Floci e o gate completo local. Assim, a restrição original de não
elevar MSRV foi revista mediante o pedido explícito do mantenedor;
`rust-version` agora declara 1.94.1. A validação de `cargo audit` local ainda
apontou advisories em `h2` 0.3 e `rustls` 0.21, trazidos pelo conector AWS
Smithy legado habilitado pelo SDK; não foi criada exceção nem declarado o gate
supply-chain aprovado. `govulncheck` não concluiu, então a verificação Go também
permanece pendente.

O decoder Go usa `github.com/pierrec/lz4/v4` 4.1.22: a 4.1.33 falha ao ler o
frame LZ4 por página do formato LTX, enquanto o teste isolado passa com 4.1.22.
Não atualizar essa dependência até que uma versão corrigida passe a mesma
conformance de formato.

## Fontes consultadas

- AWS SDK for Go v2 S3: <https://pkg.go.dev/github.com/aws/aws-sdk-go-v2/service/s3>
- Retry configuration AWS SDK for Rust: <https://docs.aws.amazon.com/sdk-for-rust/latest/dg/retries.html>
- AWS SDK for Rust S3 1.90.0 declara Rust 1.82 e expõe `if_none_match` no
  `CompleteMultipartUpload`; versão 1.100.0 declara Rust 1.86:
  <https://crates.io/crates/aws-sdk-s3/1.90.0> e
  <https://crates.io/crates/aws-sdk-s3/1.100.0> e
  <https://docs.rs/aws-sdk-s3/1.90.0/aws_sdk_s3/operation/complete_multipart_upload/struct.CompleteMultipartUploadInput.html>
