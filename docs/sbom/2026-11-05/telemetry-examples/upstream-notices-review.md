# Textos upstream: telemetria e oráculo

Revisão técnica parcial de 2026-11-05, não parecer jurídico nem pacote final de
atribuições. O projeto ainda não definiu todos os artefatos a distribuir; a
presença num grafo, em testes ou num gerador não comprova inclusão num binário.
Não publicar nem criar licença final, release ou tag com base neste documento.

## OpenTelemetry Rust 0.33.1

Os tags upstream `opentelemetry-0.33.1` e `opentelemetry_sdk-0.33.1` resolvem
ambos ao commit `372ebbc74d4dba71bb9820732f57f7316b4e7a1e`. O
[LICENSE na raiz desse commit](https://github.com/open-telemetry/opentelemetry-rust/blob/372ebbc74d4dba71bb9820732f57f7316b4e7a1e/LICENSE)
contém Apache License 2.0; SHA-256 do texto obtido:
`c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`.
A árvore upstream consultada para cada tag não listou arquivos chamados
`NOTICE` ou `LICENSE` além desse `LICENSE` raiz. Os manifests das duas crates
no registry declaram `Apache-2.0`, mas suas cópias locais não possuem arquivo
`LICENSE*`/`NOTICE*` na raiz da crate. Um arquivo upstream localizado no
repositório não comprova sozinho que o pacote publicado contém os mesmos bytes
ou cobre todo código de terceiros; revisar o pacote e o artefato final.

Após o download para o teste do exemplo do candidato `413e08d`, as duas
archives `.crate` do cache foram conferidas diretamente contra o `Cargo.lock`
incluído no candidato: `opentelemetry` 0.33.1 tem SHA-256
`8c1f1a6ac9c875a4941e4c7c085b61e8f4daa326de03b4701768c3491a6ad9ee`
e 50 membros; `opentelemetry_sdk` 0.33.1 tem SHA-256
`372a912db2974d1990207006fd0e3aa0ef3fd4ad3be43c65cfaadf2206866061`
e 95 membros. Os dois hashes coincidem com o lockfile. A busca por basename
iniciado por `LICENSE`, `NOTICE` ou `COPYING`, sem distinção de maiúsculas,
em todos os caminhos dessas duas archives não encontrou correspondências.
Isso amplia a busca anterior limitada à raiz; não exclui avisos embutidos ou
com outros nomes nem comprova equivalência integral com a árvore upstream. Os
pacotes baixados não integram a archive-fonte do candidato; a revisão jurídica
e dos textos para o artefato final permanece pendente.

No exemplo Rust, `opentelemetry` é dependência normal; `opentelemetry_sdk` é
dev-dependency e não consta como tal no SBOM `rust-telemetry.cdx.json`. Os testes
usam seu exporter em memória. A aplicabilidade ao pacote depende de distribuir
fontes de teste, binários de teste, ou apenas outro artefato.

## Prometheus e OpenTelemetry Go

Nos caches dos módulos fixados do exemplo, os quatro módulos Prometheus abaixo
têm texto Apache-2.0 e cada um tem seu próprio `NOTICE`. Foram lidos os textos
completos, não somente nomes dos arquivos:

| Módulo | SHA-256 do `NOTICE` | Conteúdo identificado |
|---|---|---|
| `client_golang v1.25.0` | `8548bfb2bc1913396df38c548a6ec60b8dbcac3b0800485a40569c8bdd184471` | Prometheus, SoundCloud, perks e suporte Go a Protocol Buffers. |
| `client_model v0.6.3` | `6c79faa15168885fb88a316ae0df18f486deafddefdc16826cdc56dfbd421e24` | Prometheus e SoundCloud. |
| `common v0.72.0` | `600244c8052c8c1d307043fafacbc83b429c171dd8e08f5a537c0a54014585ee` | Prometheus e SoundCloud. |
| `procfs v0.22.0` | `497fcdb9fae55924c12f55f05cbe83a096662e5c591d79b74e4a8f23bab5ffee` | Prometheus e SoundCloud. |

Os arquivos `LICENSE` desses quatro módulos têm SHA-256
`c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`.
Os módulos Go OpenTelemetry `otel`, `metric`, `sdk`, `sdk/metric` e `trace`
v1.47.0 têm `LICENSE` Apache-2.0 com SHA-256
`1ae07514be1d7bb33f0698f8d91fb51b8b9fe1463157ec1c72081a49b9bc6f40`;
não foi encontrado `NOTICE*` na raiz dessas cópias. Os hashes diferentes dos
textos demonstram que não devem ser tratados como intercambiáveis sem examinar
seu conteúdo. A busca em raiz não cobre avisos aninhados ou dependências.

Caso sejam aplicáveis à distribuição escolhida, preservar textos originais de
licença, copyright, condições e notices pertinentes, incluindo atribuições de
terceiros; o resumo nesta página **não substitui** os textos de notice. Revisar
como os termos Apache-2.0, se aplicáveis, tratam LICENSE, avisos de alteração e
NOTICE para cada artefato, sem presumir que todo item do grafo está incorporado.

## Oráculo LTX

O tag `superfly/ltx v0.4.0` resolve ao commit
`2af9b0cb7a6eebfb59c2ca76acc4ae3adf4b6a09`. Seu
[LICENSE upstream](https://github.com/superfly/ltx/blob/2af9b0cb7a6eebfb59c2ca76acc4ae3adf4b6a09/LICENSE)
é Apache-2.0 e tem o mesmo SHA-256 `c71d239d…f35d0ab4` do arquivo
`LICENSE` na cópia local do módulo; a árvore upstream consultada não listou
arquivo denominado `NOTICE`/`LICENSE` além do `LICENSE` raiz. O oráculo é um
módulo de conformance separado, usado como ferramenta de teste e geração, não
uma dependência automaticamente comprovada dos binários de biblioteca. Seus
termos, dependências e avisos de terceiros devem ser revisados se fontes, ferramenta
ou arquivos derivados forem distribuídos. A licença do gerador não determina por
si só os direitos sobre fixtures; ver `../fixture-provenance-review.md`.

## Fechamento pendente

Faltam definir os artefatos/targets/features e respectivos SBOMs, examinar
notices e licenças transitivos e nativos efetivamente incluídos, conferir arquivos
aninhados e dev-dependencies quando distribuídas, preparar e revisar os textos
completos de atribuição e confirmar direitos das fixtures e contribuições. Os
SBOMs de telemetria são inventários parciais e a evidência automática `modernc`
continua controversa. A revisão legal e a autorização de publicação cabem ao
mantenedor; estas observações não as substituem.
