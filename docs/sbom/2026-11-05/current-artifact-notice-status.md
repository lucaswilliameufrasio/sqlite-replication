# Estado dos notices por artefato — revisão local parcial

Esta matriz registra evidência disponível, não aprovação jurídica, licença final
ou autorização para distribuir. As alterações de código e testes do checkout
estão sem commit; nenhum candidato gerado anteriormente representa automaticamente
esse estado. Os SBOMs originais e suas cópias em quarentena foram preservados.

## Conferência de entradas

Os SHA-256 abaixo foram conferidos no checkout atual e coincidem com os
registrados em `README.md` para a triagem anterior:

| Entrada | SHA-256 |
|---|---|
| `go/go.mod` | `f5bfda4168e3619d018e4a284d245fabcab41f13f8921ac1b6262d1548d1c8b2` |
| `go/go.sum` | `208cbd4dee9e3ec082d9dc710bec018ac04915db306fe4555813be5f59f1caa7` |
| `rust/Cargo.toml` | `94f7a02b159a500b1da2f0f8d4ab864a30fc819be31c05039332c6d5dfd5f962` |
| `rust/Cargo.lock` | `7083d5346548c1fea412bd9cdb33db7dda1d1d81bf58c70f02654d4d8cd3bb96` |

Essas igualdades não comprovam composição efetiva, seleção de features ou
dependências de outros workspaces; tampouco verificam o conteúdo dos SBOMs.

## Estado por possível artefato

| Artefato | Evidência existente | Pendência antes de distribuição |
|---|---|---|
| Fontes Go e consumidores | SBOMs Go e `native-notices-review.md`, escopo Linux amd64. | Identificar conteúdo efetivo e revisar textos integrais e avisos aplicáveis; manter em quarentena a classificação automática controversa de `modernc`. |
| Workspace Rust com fixtures | `rust-workspace-candidate-notice-matrix.md` e inventário de archives anteriores. | Gerar e examinar candidato do estado escolhido, verificar arquivos e avisos incluídos e revisar textos integrais; candidatos antigos não cobrem mudanças posteriores. |
| Crates Rust individuais | `distribution-candidate-inventory.md` relata dependências locais e fixtures ausentes em pacotes isolados. | Resolver estratégia de empacotamento e verificar cada pacote extraído; não presumir que o workspace validado prove crates independentes. |
| Binários Go/Rust | `native-notices-review.md`, `sqlite-rust-transitive-notices.md` e `s3-aws-lc-transitive-notices.md` são triagens de configurações anteriores. | Definir binários, targets e features; verificar componentes efetivos, SQLite, AWS-LC e avisos correspondentes. |
| Exemplos de telemetria | `telemetry-examples/` tem inventários separados; exemplo Rust é workspace separado. | Definir inclusão, revisar suas dependências e confirmar os termos próprios do exemplo. |
| Fixtures e documentos SBOM | `fixture-provenance-review.md`, inventário anterior e originais/quarentena preservados. | Conferir os arquivos efetivamente incluídos e atribuições aplicáveis; confirmação do usuário sobre direitos de contribuições e fixtures não substitui revisão dos avisos de terceiros. |

As revisões identificadas são parciais. A definição do conjunto final de
artefatos e a aprovação de distribuição continuam distintas da validação local;
nenhuma publicação, tag, licença final ou push está autorizada por este registro.

## Candidato Rust local posterior

Duas gerações do checkout modificado produziram archives idênticas de SHA-256
`f8313da0cdb1c14015a3cf9410d55edd453a7d75790bd1e5a15653f78b495131`
em `/tmp/opencode/sqlite-notice-audit-Sm6HsqcO/`. Inspeção sem extração verificou
122 hashes, dois arquivos de controle, ausência de nomes duplicados, caminhos
absolutos ou `..`, e de membros não regulares. Três arquivos com `quarantined`
no nome estão presentes. Nenhum membro tem basename exatamente `LICENSE`,
`NOTICE` ou `COPYING`; essa busca não identifica avisos embutidos ou outros nomes.
Posteriormente, a archive foi verificada e extraída em diretório isolado em
`~/git-worktrees/sqlite-replication/candidate-f8313da-validation-20261106/`.
Com Cargo 1.99.1, cache local offline e target novo, `cargo test --offline
--locked --workspace --all-features` passou nos testes não ignorados: 63 do
core e 30 do coordinator, entre os demais. Os testes opt-in de Floci, CP-01 e
interoperabilidade entre processos ficaram ignorados nessa execução. O exemplo
de telemetria é workspace separado e não está coberto por esse comando. Isso
não demonstra instalação offline em outro host nem aprova notices ou distribuição.
Esta anotação é posterior à geração e não está na archive.

### Avisos Prometheus do exemplo Go

No grafo normal do exemplo Go isolado em `linux/amd64`, foram selecionados
`client_golang v1.25.0`, `client_model v0.6.3`, `common v0.72.0` e
`procfs v0.22.0`. Os quatro arquivos `NOTICE` na raiz dos respectivos módulos
foram lidos integralmente no cache local; seus SHA-256 são:

| Módulo | SHA-256 do `NOTICE` |
|---|---|
| `client_golang` | `8548bfb2bc1913396df38c548a6ec60b8dbcac3b0800485a40569c8bdd184471` |
| `client_model` | `6c79faa15168885fb88a316ae0df18f486deafddefdc16826cdc56dfbd421e24` |
| `common` | `600244c8052c8c1d307043fafacbc83b429c171dd8e08f5a537c0a54014585ee` |
| `procfs` | `497fcdb9fae55924c12f55f05cbe83a096662e5c591d79b74e4a8f23bab5ffee` |

Os quatro creditam The Prometheus Authors e mencionam SoundCloud; o `NOTICE`
de `client_golang` também identifica perks e suporte Go a Protocol Buffers. A
aplicabilidade dos avisos e dos textos de licença depende dos arquivos e da
forma de distribuição efetivos; seleção pelo compilador não comprova inclusão
no artefato final. Os avisos não foram copiados para um pacote de distribuição,
e outros avisos embutidos e transitivos continuam sujeitos a revisão.

O exemplo Rust de telemetria incluído nessa mesma archive foi testado
separadamente com `cargo test --offline --locked --all-features`: passaram os
quatro testes de integração. Usou o mesmo `fresh-target` criado para esta
extração e o cache local de dependências; não é prova de instalação offline em
outro host ou de aprovação de seus notices.

Os SHA-256 de `go.mod` e `go.sum` do exemplo Go isolado e de `Cargo.toml` do
exemplo Rust extraído coincidem com as entradas da triagem
`telemetry-examples/README.md`. Porém o `Cargo.lock` Rust extraído tem SHA-256
`fc290e4d1bed61333112badf073e327d82b4a4c20d761c6f9d1e2d0daaf90c5f`,
diferente do hash `2cd20995…147d85f` registrado naquela triagem. A comparação
específica mostrou que as versões e checksums ali examinados de
`opentelemetry 0.33.1`, `opentelemetry_sdk 0.33.1`, `prometheus 0.14.0`,
`protobuf 3.7.2` e `protobuf-support 3.7.2` permanecem iguais; `opentelemetry_sdk`
não figura no SBOM Rust anterior, conforme já documentado para dependências de
teste. Isso não estabelece que o restante dos grafos ou avisos seja igual: gerar
e revisar inventário atualizado para qualquer artefato escolhido antes de
considerar sua revisão concluída.
Comparando pares nome/versão, o lockfile tem 252 entradas de registry e 39
delas não aparecem entre os componentes do SBOM histórico. A comparação é de
escopos diferentes: lockfile inclui dependências de teste e outros targets;
os 39 não são, por isso, automaticamente componentes distribuídos ou prova
individual de deriva. O inventário atualizado deve selecionar o alvo e o
artefato específicos e tratar separadamente testes e demais plataformas.

### Seleção atual dos exemplos de telemetria

Nas cópias isoladas já testadas, em Linux x86-64 e sem rede externa:

* Rust: `cargo tree --offline --locked --all-features --target
  x86_64-unknown-linux-gnu --prefix none --format '{p}' -e normal,build`
  produziu 215 pares únicos de pacote/versão, incluindo o pacote raiz e os
  locais. Comparados por nome/versão com `rust-telemetry.cdx.json` histórico,
  somente o próprio pacote raiz não apareceu como componente. O comando com
  `-e normal,build,dev` produziu 224 pares: os nove adicionais são
  `futures-executor 0.3.34`, `futures-macro 0.3.34`, `getrandom 0.3.4`,
  `opentelemetry_sdk 0.33.1`, `ppv-lite86 0.2.21`, `rand 0.9.5`,
  `rand_chacha 0.9.0`, `rand_core 0.9.5` e `zerocopy 0.8.63`.
  Comparar somente nome/versão não verifica checksums, features, relações,
  licenças, código nativo nem bytes efetivamente vinculados.
* Go: `go list -mod=readonly -deps` para o módulo do exemplo selecionou 32
  módulos com versão, inclusive `sqlite-replication v0.0.0` substituído pela
  cópia local. Com `-test`, selecionou 38: os seis adicionais foram
  `github.com/go-logr/logr v1.4.4`, `github.com/go-logr/stdr v1.2.2`,
  `go.opentelemetry.io/auto/sdk v1.2.1`, `go.opentelemetry.io/otel/log v1.47.0`,
  `go.opentelemetry.io/otel/sdk v1.47.0` e
  `go.opentelemetry.io/otel/sdk/metric v1.47.0`. O resultado depende do alvo
  `linux/amd64`, do Go 1.27.3 e do replacement local; não é link map.

Estas consultas reproduzíveis são inventários de seleção, não SBOMs CycloneDX
novos nem revisão completa dos textos. Para cada artefato final ainda é preciso
gerar SBOM vinculado ao build e revisar notices e fontes relevantes; os SBOMs
originais e em quarentena permanecem intactos.

#### Textos adicionais selecionados para testes Go

Os módulos `github.com/go-logr/logr v1.4.4` e `github.com/go-logr/stdr v1.2.2`
foram selecionados apenas ao incluir testes no exemplo Go isolado. Os arquivos
`LICENSE` das cópias locais foram lidos integralmente e contêm Apache 2.0; os
SHA-256 são, respectivamente,
`b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1` e
`c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`.
Os cabeçalhos examinados em `logr.go` e `stdr.go` creditam 2019 The logr
Authors e referenciam Apache 2.0. A presença na seleção de testes não comprova
inclusão em qualquer artefato final; outros arquivos, avisos embutidos e a
aplicabilidade dos textos seguem sujeitos a revisão. Nenhum notice foi aprovado.

Na mesma seleção de testes Go, `go.opentelemetry.io/otel/log v1.47.0`,
`go.opentelemetry.io/otel/sdk v1.47.0` e `go.opentelemetry.io/otel/sdk/metric
v1.47.0` possuem `LICENSE` com SHA-256 idêntico
`1ae07514be1d7bb33f0698f8d91fb51b8b9fe1463157ec1c72081a49b9bc6f40`.
O texto foi lido integralmente: após Apache 2.0, inclui também um texto BSD de
três condições com copyright 2009 The Go Authors e seu disclaimer. A licença
na raiz de `go.opentelemetry.io/auto/sdk v1.2.1` tem SHA-256
`c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`,
idêntico ao texto Apache 2.0 de `stdr` lido acima. As árvores examinadas não
foram integralmente analisadas por avisos embutidos; os textos originais não
são substituídos por este resumo. O conjunto de testes selecionado não prova
que esses módulos integrem o artefato a distribuir, e a aplicabilidade dos
avisos permanece por decidir por artefato.

Uma busca literal por `The Go Authors` nos arquivos `.go` das três árvores
locais `otel/log`, `otel/sdk` e `otel/sdk/metric` acima não retornou resultados.
Uma consulta separada por copyright encontrou cabeçalhos de The OpenTelemetry
Authors, mas foi truncada. A ausência da frase nos arquivos `.go` não explica
por que o texto BSD consta no `LICENSE`, não exclui outros arquivos ou avisos
nem permite eliminar o texto de um pacote de distribuição futuro.

Busca recursiva nas quatro árvores locais `auto/sdk`, `otel/log`, `otel/sdk` e
`otel/sdk/metric` identificou somente o `LICENSE` de raiz de cada módulo entre
os basenames iniciados por `LICENSE`, `NOTICE` ou `COPYING`, sem distinção de
maiúsculas; os quatro hashes correspondem aos registrados acima. A busca não
examina avisos embutidos ou arquivos com outros nomes e não substitui a revisão
de licenças e atribuições para a composição final de cada artefato.

#### Textos adicionais selecionados para testes Rust

Na cópia local do registry, os manifests de `futures-executor 0.3.34`,
`futures-macro 0.3.34`, `getrandom 0.3.4`, `ppv-lite86 0.2.21`, `rand 0.9.5`,
`rand_chacha 0.9.0` e `rand_core 0.9.5` declaram `MIT OR Apache-2.0`; cada
raiz examinada contém arquivos `LICENSE-MIT` e `LICENSE-APACHE`.
`zerocopy 0.8.63` declara `BSD-2-Clause OR Apache-2.0 OR MIT` e contém os
três arquivos correspondentes. `opentelemetry_sdk 0.33.1` declara Apache-2.0,
mas não tinha arquivo com prefixo `LICENSE`, `NOTICE` ou `COPYING` na raiz;
a revisão prévia da archive e do texto upstream está em
`telemetry-examples/upstream-notices-review.md`.

Foram relidos integralmente três textos locais: o MIT de `futures-executor`
credita Alex Crichton e The Tokio Authors (SHA-256
`6652c868f35dfe5e8ef636810a4e576b9d663f3a17fb0f5613ad73583e1b88fd`); o MIT
de `rand` credita Developers of the Rand project e The Rust Project Developers
(`209fbbe0ad52d9235e37badf9cadfe4dbdc87203179c0899e738b39ade42177b`);
o BSD de `zerocopy` credita The Fuchsia Authors
(`83c1763356e822adde0a2cae748d938a73fdc263849ccff6b27776dff213bd32`).
Os textos completos incluem condições de preservação e disclaimers; este resumo
não os substitui. Uma busca recursiva nas nove árvores locais, por basenames
iniciados por `LICENSE`, `NOTICE` ou `COPYING`, sem distinção de maiúsculas,
não encontrou arquivos aninhados. Ela não examina avisos embutidos ou nomes
diferentes. A leitura dos demais textos continua pendente. Estas dependências
adicionais foram selecionadas ao incluir
desenvolvimento; isso não prova sua inclusão no artefato a distribuir nem decide
qual alternativa de licença se aplica. Não houve aprovação de notices.

Também foram lidos integralmente os textos MIT locais de `getrandom`,
`ppv-lite86` e `zerocopy`: creditam respectivamente The rust-random Project
Developers e The Rust Project Developers; The CryptoCorrosion Contributors;
e The Fuchsia Authors. Os SHA-256 respectivos são
`29e9fe5074bd27e0e5d5d110394fbbcd841baee2651a3c4b4560a632702cede4`,
`4cada0bd02ea3692eee6f16400d86c6508bbd3bafb2b65fed0419f36d4f83e8f` e
`1a2f5c12ddc934d58956aa5dbdd3255fe55fd957633ab7d0d39e4f0daa73f7df`.
Os textos MIT de `futures-macro`, `rand_chacha` e `rand_core` têm os mesmos
hashes, respectivamente, dos textos de `futures-executor` e `rand` já lidos;
isso identifica bytes iguais no cache, sem validar sua procedência upstream.
Nenhuma alternativa de licença foi selecionada: as opções Apache e BSD
pertinentes ainda devem ser avaliadas, junto com avisos embutidos e escopo real
de cada artefato. Não se trata de aprovação jurídica ou de distribuição.

Foram também lidos integralmente os textos `LICENSE-APACHE` locais de
`futures-executor 0.3.34` (SHA-256
`275c491d6d1160553c32fd6127061d7f9606c3ea25abfad6ca3f6ed088785427`) e
`rand 0.9.5` (`35242e7a83f69875e6edeff02291e688c97caafe2f8902e4e19b49d3e78b4cab`).
O primeiro inclui apêndice com créditos de Alex Crichton e The Tokio Authors;
o segundo termina nos termos, sem esse apêndice. A seção de redistribuição
trata, entre outros pontos, da entrega da licença, de arquivos modificados e
dos avisos aplicáveis, inclusive quando existe `NOTICE`. Os resumos não
substituem os textos. Os arquivos Apache locais de `futures-macro` e
`rand_chacha` têm, respectivamente, os mesmos hashes desses dois textos; isso
não determina sua aplicação a um artefato específico. Os demais textos Apache,
avisos embutidos e a procedência upstream ainda exigem revisão.

O `LICENSE-APACHE` de `getrandom 0.3.4` também foi lido integralmente no cache;
seu SHA-256 é `aaff376532ea30a0cd5330b9502ad4a4c8bf769c539c87ffe78819d188a18ebf`.
O arquivo contém os termos Apache 2.0 e um apêndice com campos exemplificativos
de copyright, não uma atribuição preenchida. A referência não substitui os
avisos da própria crate ou a revisão do artefato efetivamente distribuído; a
licença alternativa aplicável não foi selecionada.

O `LICENSE-APACHE` de `ppv-lite86 0.2.21` foi lido integralmente no cache;
seu SHA-256 é `0218327e7a480793ffdd4eb792379a9709e5c135c7ba267f709d6f6d4d70af0a`.
Além dos termos Apache 2.0, o arquivo inclui apêndice com copyright 2019 The
CryptoCorrosion Contributors. A leitura não substitui a revisão dos avisos
dos arquivos incluídos no artefato final, nem seleciona uma das alternativas
de licença declaradas pela crate.

Os textos `LICENSE-APACHE` de `rand_core 0.9.5` e `zerocopy 0.8.63` também
foram lidos integralmente no cache; seus SHA-256 são, respectivamente,
`6df43f6f4b5d4587f3d8d71e45532c688fd168afa5fe89d571cb32fa09c4ef51` e
`9d185ac6703c4b0453974c0d85e9eee43e6941009296bb1f5eb0b54e2329e9f3`.
O primeiro contém os termos e instruções do apêndice sem aviso preenchido; o
segundo inclui no apêndice copyright 2023 The Fuchsia Authors. Esses arquivos
são alternativas declaradas pelos respectivos manifests, não uma escolha de
licença ou pacote de notices aprovado. Sua aplicabilidade depende dos arquivos
e artefatos efetivamente distribuídos.

Uma busca limitada a arquivos `.rs` nas cópias de `getrandom 0.3.4` e
`ppv-lite86 0.2.21`, pelos termos `copyright`, `SPDX-License-Identifier`,
`licensed under` ou `license:`, sem distinção de maiúsculas, não retornou
ocorrências. A mesma busca em `rand 0.9.5` e `zerocopy 0.8.63` retornou mais
resultados que o limite da consulta. Por exemplo, `rand/src/prelude.rs` credita
Developers of the Rand project e referencia Apache-2.0 ou MIT; `zerocopy/build.rs`
credita The Fuchsia Authors e identifica BSD-2-Clause, Apache-2.0 ou MIT. Não
foram examinados todos os resultados, outros tipos de arquivos ou avisos sem
esses termos. Esses fontes estão no cache de dependências, não foram identificados
como arquivos vendorizados na archive Rust e sua seleção para testes não prova
inclusão em distribuição.

### Dependências LTX selecionadas no candidato Rust

Na extração da archive `f8313da0…b495131`, `cargo tree --offline --locked -p
sqlite-replication-ltx -e normal,features --target x86_64-unknown-linux-gnu`
selecionou `lz4_flex 0.14.0` e `twox-hash 2.1.5` com `xxhash32`. Foram relidos
integralmente os textos MIT no cache local: `lz4_flex/LICENSE`, copyright 2020
Pascal Seitz, SHA-256 `0982c33390159842ecce8e9d6ce2e5e39961fe0e0ffbb2eec39c9ba46db6db10`;
e `twox-hash/LICENSE.txt`, copyright 2015 Jake Goulding, SHA-256
`f59bcb1c02654665f78a5f290bae76b6358137675f6459f51e6d474a4623d8ea`.
Ambos contêm condições de preservação do aviso e disclaimers completos; o
resumo não é substituto dos textos. Os hashes coincidem com a revisão anterior
`ltx-transitive-notices.md`, inclusive a questão ainda aberta do trecho da
especificação LZ4 reproduzido em comentário. A archive-fonte não vende essas
dependências; o grafo não prova quais objetos integram um binário futuro nem
resolve as obrigações de cada formato de distribuição.

### Adapter SQLite no candidato Rust

Na extração `f8313da0…b495131`, o grafo normal e de build para
`x86_64-unknown-linux-gnu` selecionou `rusqlite 0.41.1`, com `backup` e
`bundled`, e `libsqlite3-sys 0.39.1` com SQLite incorporado. Os textos `LICENSE`
dos dois wrappers foram relidos integralmente no cache: ambos identificam
copyright 2014 The rusqlite developers e têm SHA-256
`c10c1f27337546471e5f7e4e97fdd398b35b9d4e126115dcd22de8d8e65abf6f`.
O texto MIT requer preservar o aviso e a permissão conforme seus termos e
inclui disclaimer; este resumo não o substitui. A amalgamação local `sqlite3.c`
tem SHA-256 `0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52`,
igual ao registrado em `sqlite-rust-transitive-notices.md`. O LICENSE do wrapper
não determina sozinho os termos da amalgamação ou de todos os demais arquivos.
Estas dependências não são vendorizadas na archive-fonte; a seleção por Cargo
não comprova conteúdo de um binário final. Continuam necessárias as revisões
por artefato dos textos SQLite, componentes nativos e dependências de build.

### S3/TLS no candidato Rust

Na extração `f8313da0…b495131`, `cargo tree --offline --locked -p
sqlite-replication-storage-s3 -i aws-lc-sys --target x86_64-unknown-linux-gnu`
confirmou o caminho selecionado via `aws-sdk-s3 1.157.0`,
`aws-smithy-runtime 1.16.2`, `aws-smithy-http-client 1.5.2`, `rustls 0.23.46`,
`aws-lc-rs 1.19.0` e `aws-lc-sys 0.47.0`. A archive da última crate no cache
tem SHA-256 `8bfbe32ac36b3c428696ce315464181bc4029d689fd58e3f2a4e58c252242f52`,
igual ao checksum do lockfile extraído e ao registrado em
`s3-aws-lc-transitive-notices.md`. A revisão anterior descreve o LICENSE
composto e textos upstream para imports com licenças ausentes da crate; suas
pendências de aplicabilidade e composição de binários permanecem abertas.
A archive-fonte do candidato não inclui a archive da dependência; este grafo e
checksum não constituem link map nem aprovação de notices ou distribuição.

## Cópia Go local para teste

Uma cópia de investigação do checkout modificado foi criada sem sobrescrever
arquivos em `~/git-worktrees/sqlite-replication/go-source-candidate-validation-20261106/`.
Inclui `go/` e `conformance/fixtures/`, inclusive arquivos não rastreados não
ignorados, e não define um pacote de distribuição. Seus 127 arquivos foram
verificados contra `SHA256SUMS`, cujo SHA-256 é
`f4fa5779112adb7d5e6e5541cb932629f37028f8c9e413ecbe1fd09bab621d13`.
Na cópia, com Go 1.27.3 e dependências do cache local, `GOPROXY=off go test
-race -count=1 ./...` passou nos pacotes do módulo principal. Workspaces Go
separados, Floci e interoperabilidade opt-in não foram executados nessa cópia;
não se demonstrou instalação offline em outro host nem aprovação de notices.

O workspace Go separado `go/examples/coordinator-telemetry`, incluído na mesma
cópia, também passou `GOPROXY=off go test -race -count=1 ./...` com dependências
do cache local. Essa execução não incorpora nem aprova notices de terceiros.

Inspeção dos nomes registrados no manifesto dessa cópia encontrou 77 arquivos
sob `go/` e 50 sob `conformance/fixtures/`, nenhum com basename iniciado por
`LICENSE`, `NOTICE` ou `COPYING` (comparação sem distinção de maiúsculas).
A cópia não inclui esta documentação de SBOM, nem um pacote de notices de
terceiros. A busca por nomes não verifica avisos embutidos nem estabelece quais
textos ou atribuições seriam exigidos em eventual distribuição. Seu manifesto
e testes não equivalem a uma aprovação para distribuição.

Uma busca textual sem distinção de maiúsculas nos arquivos `.go` dessa cópia,
incluindo o exemplo de telemetria, por `copyright`, `SPDX-License-Identifier`
ou `licensed under` não encontrou ocorrências. A busca não identifica código
copiado sem esses marcadores, outras formas de aviso ou direitos de terceiros;
não substitui a revisão dos textos aplicáveis às dependências selecionadas.
