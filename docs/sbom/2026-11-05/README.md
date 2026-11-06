# SBOM e triagem de licenças — 2026-11-05

Retrato local de dependências, **não** SBOM de um release ou aprovação jurídica.
Base: commit `3553f91` com alterações locais em documentação e teste de
coordenador; manifests e lockfiles examinados não tinham alterações pendentes.
O prefixo local de caminhos nos BOMs Rust foi normalizado para
`file:///workspace/rust/`; esses caminhos são identificadores, não downloads.

## Artefatos e método

Os exemplos isolados de telemetria, criados posteriormente, têm inventários e
limitações próprios em [`telemetry-examples/`](telemetry-examples/README.md).
Uma inspeção posterior das listas de arquivos candidatas dos crates está em
[`distribution-candidate-inventory.md`](distribution-candidate-inventory.md).
Uma revisão parcial da cadeia transitiva LTX Rust está em
[`ltx-transitive-notices.md`](ltx-transitive-notices.md).
O grafo SQLite Rust foi triado separadamente em
[`sqlite-rust-transitive-notices.md`](sqlite-rust-transitive-notices.md).
A triagem parcial S3/TLS e AWS-LC está em
[`s3-aws-lc-transitive-notices.md`](s3-aws-lc-transitive-notices.md).

| Arquivo | Escopo | Componentes |
|---|---|---:|
| `go.cdx.json` | módulo Go, sem dependências de teste selecionadas | 22 |
| `go-with-tests.cdx.json` | mesmo módulo, incluindo dependências de teste | 29 |
| `go-oracle-with-tests.cdx.json` | módulo separado `conformance/reference`, com testes | 2 |
| `sqlite-replication-core.cdx.json` | crate core Rust e dependências | 205 |
| `sqlite-replication-ltx.cdx.json` | crate LTX | 2 |
| `sqlite-replication-spool.cdx.json` | crate spool | 28 |
| `sqlite-replication-sqlite.cdx.json` | crate SQLite | 13 |
| `sqlite-replication-storage-fs.cdx.json` | crate filesystem | 23 |
| `sqlite-replication-storage-s3.cdx.json` | crate S3 | 189 |
| `sqlite-replication-wal.cdx.json` | crate WAL | 3 |

Os Go foram gerados com CycloneDX `cyclonedx-gomod` **1.12.0**, subcomando
`mod -json -licenses -type library`, adicionando `-test` nos dois arquivos
indicados. O módulo principal foi analisado para Linux amd64 com Go 1.27.3; o
oráculo tem módulo próprio e não integra os runtimes. As licenças Go constam
como *evidence*, não como licenças juridicamente concluídas. O comando `go list
-m all` inclui ainda módulos do grafo que não aparecem nos BOMs gerados; não
interpretar os BOMs como inventário de todos os possíveis builds e ferramentas.

Os Rust foram gerados com `cargo-cyclonedx` **0.5.9**, com `--format json
--all --all-features --target x86_64-unknown-linux-gnu --spec-version 1.5`,
para cada crate do workspace e os manifests/lockfile presentes. A ferramenta
documenta exclusão de dev-dependencies; não representa todos os targets,
consumidores ou recursos nativos de build. Os sete arquivos são visões
sobrepostas, não quantidades somáveis de dependências únicas.

Os binários de release foram obtidos temporariamente e comparados com SHA-256
informados pelos respectivos releases: `cyclonedx-gomod` arquivo
`004b9f5cc595b797fb5423e2ae4c97bcf0f18c712ed2faee1640b09e5efd6d15`;
`cargo-cyclonedx` arquivo musl
`9bd3e599314f50810c9d98b8b68a617ff9d3cc20873968d90b29d121f6b226ff`.
Sem instalação global; execução em Docker rootless isolado, sem rede externa e
com caches montados somente para leitura. Os dez JSONs finais passaram no
`cyclonedx validate --fail-on-errors` da CLI CycloneDX **0.33.1**, usando
schema 1.6 para Go e 1.5 para Rust, após normalização dos caminhos. O binário de
validação tinha SHA-256
`bfc8b2538da86fe239bc53658bbb63c1c8c510a293c1e6891aa5bea5d3c58746`.
Validade estrutural não valida cobertura, exatidão jurídica nem ausência de
vulnerabilidades.

Entradas, SHA-256:

- `go/go.mod`: `f5bfda4168e3619d018e4a284d245fabcab41f13f8921ac1b6262d1548d1c8b2`
- `go/go.sum`: `208cbd4dee9e3ec082d9dc710bec018ac04915db306fe4555813be5f59f1caa7`
- `conformance/reference/go.mod`: `443564fcf9d79bd9c70d67da7a7179e7b688229951ccfa8a40d472879dd0cf19`
- `conformance/reference/go.sum`: `f61d5d5df530950fcf25f22ee76df21beca52d60394c446ed01a9aa3aa688240`
- `rust/Cargo.toml`: `94f7a02b159a500b1da2f0f8d4ab864a30fc819be31c05039332c6d5dfd5f962`
- `rust/Cargo.lock`: `7083d5346548c1fea412bd9cdb33db7dda1d1d81bf58c70f02654d4d8cd3bb96`

## Triagem inicial, não parecer

**Correção importante:** a análise posterior dos arquivos originais encontrou
divergências na evidência automática de licença dos módulos `modernc.org/sqlite`
e `modernc.org/libc`. Os JSONs permanecem como gerados, mas suas licenças não
devem ser usadas como classificação definitiva. Detalhes e escopo dos binários
examinados: [`native-notices-review.md`](native-notices-review.md).

Uma investigação reproduziu a seleção por maior confiança do gerador e produziu
duas cópias com evidência controversa em quarentena, sem afirmar licença
substituta: `go-evidence-quarantined.cdx.json` e
`go-with-tests-evidence-quarantined.cdx.json`. Veja o método em
[`license-detector-investigation.md`](license-detector-investigation.md). Os
originais são preservados; nenhuma das versões aprova distribuição.

- O gerador Go reportou ausência de licença detectada nos componentes raiz do
  projeto e do oráculo local. A ADR 0014 prevê Apache-2.0 para o projeto, mas
  proíbe criar `LICENSE` ou publicar antes da revisão; metadado pretendido não
  substitui o arquivo nem atribuições. No BOM Go com testes, os 29 componentes
  têm evidência declarada: 19 Apache-2.0, 4 MIT e 6 BSD-3-Clause; verificar os
  próprios textos, avisos e escopo antes de distribuição.
- O BOM Rust do core contém 205 componentes com algum campo de licença. Entre
  as expressões declaradas há Apache/MIT, ISC, Zlib, Unicode-3.0, BSL-1.0 e
  exceção LLVM. Essas expressões não provam conformidade. `aws-lc-sys` possui
  licença composta e código de múltiplas origens; seus avisos e componentes
  embutidos precisam de revisão. `libsqlite3-sys` declara MIT para o crate e
  inclui uma amalgamação SQLite 3.54.1: examinar seus termos e avisos próprios,
  sem inferir a licença do código C apenas pela licença do wrapper.
- As fixtures LTX são descritas em `conformance/fixtures/README.md` como dados
  determinísticos gerados por `superfly/ltx` **v0.4.0**, cuja cópia no cache
  contém texto Apache-2.0; WAL é descrito como gerado localmente pelo script em
  `conformance/fixtures/wal/README.md`. Os SHA-256 dos cinco snapshots e três
  WAL coincidiram com os valores documentados; `sha256sum --check SHA256SUMS`
  confirmou 33 arquivos da cadeia longa. Hashes demonstram identidade, não
  autoria nem licença dos dados gerados. Direitos sobre fixtures, NOTICE do
  oráculo e atribuições continuam sujeitos à revisão do mantenedor.
  Veja o inventário posterior e seus limites em
  [`fixture-provenance-review.md`](fixture-provenance-review.md).
- Não foram revisadas todas as fontes/avisos, dependências Go não selecionadas
  pelo gerador, dev-dependencies Rust ou artefatos de publicação. Não criar
  pacote legal, release ou tag com base nesta triagem; seguem necessários
  revisão de notices, direitos das fixtures e autorização específica.
