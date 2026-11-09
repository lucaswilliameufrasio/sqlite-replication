# SBOMs dos exemplos de telemetria — 2026-11-05

Inventários locais para os módulos isolados dos exemplos, **não** artefatos de
release ou parecer de conformidade. O checkout estava modificado, incluindo
exemplos e instrumentação do coordinator; o gerador Go derivou uma pseudoversão
de `f5c0984` que **não identifica os bytes não commitados**. Os arquivos de
entrada e os códigos dos exemplos devem ser reavaliados quando houver um
commit/release definido. Os SBOMs não provam conteúdo efetivamente distribuído.

| Arquivo | Escopo | Componentes |
|---|---|---:|
| `go-telemetry.cdx.json` | módulo Go, sem `-test` | 25 |
| `go-telemetry-with-tests.cdx.json` | módulo Go com `-test` | 40 |
| `go-telemetry-with-tests-evidence-quarantined.cdx.json` | cópia derivada com duas evidências controversas removidas | 40 |
| `rust-telemetry.cdx.json` | exemplo Rust, all-features, target Linux x86-64 | 220 |

Geradores: CycloneDX GoMod 1.12.0 (`mod -json -licenses -type library`, com e
sem `-test`) e cargo-cyclonedx 0.5.9 (`cyclonedx --format json --all-features
--target x86_64-unknown-linux-gnu --spec-version 1.5`). O gerador Rust não inclui
dev-dependencies: em particular, **`opentelemetry_sdk` 0.33.1, `rusqlite`,
`tempfile` e `tokio` como dependências de teste do exemplo não estão cobertos
como tais**; algumas crates podem aparecer por outros caminhos. O Go sem `-test`
não cobre as dependências exclusivas de testes. Estes inventários não são
substitutos de análise de binários, build scripts, features ou outros targets.

O cache de módulos Go foi preenchido em container isolado via proxy Go e checksum
database; geração e validação ocorreram depois sem rede externa em Docker
rootless. Para o Rust, foi usada cópia temporária da árvore e lockfile do
exemplo, com registry de dependências somente leitura e offline. Caminhos
`file:///work/source/rust/` foram substituídos consistentemente por
`file:///workspace/rust/`, que é somente identificador sintético. O shell de
login era fish, e o executor Bash. Os quatro JSONs finais passaram no
`cyclonedx validate --fail-on-errors` da CLI 0.33.1: schema 1.6 para Go e 1.5
para Rust. Uma verificação separada encontrou **zero referências não
resolvidas** em cada grafo. Validade estrutural não verifica licenças.

O Go com testes reproduz a discrepância `modernc.org/sqlite` Apache-2.0 e
`modernc.org/libc` MIT, apesar dos arquivos `LICENSE` principais BSD-3-Clause.
O original é preservado; a cópia em quarentena foi gerada pelo mesmo script
`conformance/quarantine-sbom-license-evidence.py` descrito em
`../license-detector-investigation.md`. Ela remove somente essas duas evidências
e mantém serial e timestamp do original; não afirma uma licença substituta.
O Go sem testes não contém esses dois componentes e não teve quarentena aplicada.

## Triagem de notices adicionais

Uma revisão posterior de textos e notices específicos, incluindo links e
hashes upstream do OpenTelemetry Rust e do oráculo, está em
[`upstream-notices-review.md`](upstream-notices-review.md). Ela continua parcial.

- No exemplo Go, o detector identifica os módulos Prometheus e OpenTelemetry
  selecionados como Apache-2.0. Foram conferidos arquivos `LICENSE` nos caches
  de `client_golang` 1.25.0, `client_model` 0.6.3, `common` 0.72.0,
  `procfs` 0.22.0 e dos módulos OpenTelemetry 1.47.0 selecionados. Os quatro
  módulos Prometheus também possuem `NOTICE`; o de `client_golang` menciona
  SoundCloud, perks e suporte Go a Protocol Buffers. O conteúdo aplicável deve
  ser preservado/revisado na distribuição, junto com as licenças e avisos
  transitivos. O detector também encontra MIT em `beorn7/perks` e `xxhash/v2`,
  e BSD-3-Clause em `goautoneg` e `google.golang.org/protobuf`; não são uma
  classificação jurídica completa de todos os arquivos incluídos.
- No Rust, os metadados declaram Apache-2.0 para `opentelemetry` 0.33.1 e
  `prometheus` 0.14.0; o pacote `prometheus` no cache contém `LICENSE` Apache.
  `protobuf` e `protobuf-support` 3.7.2 declaram MIT e contêm `LICENSE.txt`.
  A crate de teste `opentelemetry_sdk` 0.33.1 declara Apache-2.0 no manifesto,
  mas, assim como `opentelemetry` 0.33.1, não tinha arquivo `LICENSE*` ou
  `NOTICE*` no nível raiz da cópia do registry inspecionada. Localizar e conferir
  os textos/avisos upstream correspondentes antes de qualquer distribuição;
  essa ausência não determina por si só a licença do código.
- Os geradores alertaram que os módulos locais de exemplo e projeto não têm
  licença detectada; a ADR 0014 mantém a criação de `LICENSE` final pendente.
  As expressões automáticas são evidência, não confirmação de cobertura ou de
  direitos, e os componentes nativos e sistema continuam sujeitos à revisão
  em `../native-notices-review.md`.

## Identificação das entradas

| Arquivo | SHA-256 |
|---|---|
| `go/examples/coordinator-telemetry/go.mod` | `b524f90d1665688d25722dd0633b38fea325f7474796d456461d24703a31bc79` |
| `go/examples/coordinator-telemetry/go.sum` | `1326c986a45385380ee7dd8272de04e64723685936afd5eac15bdc0aa81458c7` |
| `rust/examples/coordinator-telemetry/Cargo.toml` | `e7b38eb0751548e335ee3b58b6b1d8d2d2dd0dfaea40cf485b3e1c159e019bd8` |
| `rust/examples/coordinator-telemetry/Cargo.lock` | `2cd209950212a950417ee984760ffcc49f686bd363fda18dc0ddfeb48147d85f` |

Continuam pendentes notices completos e SBOMs dos artefatos realmente
distribuídos, direitos das fixtures e autorização explícita de publicação.
