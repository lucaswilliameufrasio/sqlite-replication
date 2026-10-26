# ADR 0013: clientes S3 nativos por linguagem

- **Status:** Aceita para M3
- **Data:** 2026-10-26

## Contexto

M3 precisa manter adapters Go/Rust independentes, operar contra endpoint S3
configurável (inclusive Floci), suportar uploads grandes, precondições, retries
e cancelamento sem credenciais reais nos testes. O workspace Rust declara
`rust-version = "1.85"`; elevá-la por causa do SDK não foi aprovado e não é
necessário para avançar este slice.

## Decisão

- Go usa o SDK oficial AWS SDK for Go v2 (`config`, `credentials` e `service/s3`),
  com versões fixadas no `go.mod`/`go.sum` e configuração explícita do endpoint.
- Rust usa o AWS SDK oficial `aws-sdk-s3` fixado inicialmente em `1.90.0`, cuja
  versão declara MSRV 1.82 e fornece controle explícito de `If-None-Match` também
  no `CompleteMultipartUpload`. Não elevaremos a MSRV por este adapter. A versão
  fica exata até que uma atualização passe a validação da MSRV e da conformance.
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
normal de cada linguagem. Atualizações dos SDKs devem preservar a MSRV e repetir
a conformance em Floci. A escolha não habilita WAL incremental nem altera o gate
CP-01.

## Fontes consultadas

- AWS SDK for Go v2 S3: <https://pkg.go.dev/github.com/aws/aws-sdk-go-v2/service/s3>
- Retry configuration AWS SDK for Rust: <https://docs.aws.amazon.com/sdk-for-rust/latest/dg/retries.html>
- AWS SDK for Rust S3 1.90.0 declara Rust 1.82 e expõe `if_none_match` no
  `CompleteMultipartUpload`; versão 1.100.0 declara Rust 1.86:
  <https://crates.io/crates/aws-sdk-s3/1.90.0> e
  <https://crates.io/crates/aws-sdk-s3/1.100.0> e
  <https://docs.rs/aws-sdk-s3/1.90.0/aws_sdk_s3/operation/complete_multipart_upload/struct.CompleteMultipartUploadInput.html>
