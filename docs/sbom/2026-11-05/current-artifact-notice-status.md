# Estado dos notices por artefato — revisão local parcial

Esta matriz registra evidência disponível, não aprovação jurídica, licença final
ou autorização para distribuir. Os candidatos foram gerados antes dos commits
`578f56e`, `c70c1dd` e `22c8ba0`; a comparação posterior dos arquivos de código
e fixtures está registrada abaixo. A documentação desta matriz foi ampliada
depois da geração da archive Rust, que não representa automaticamente a árvore
inteira atual. Os SBOMs originais e suas cópias em quarentena foram preservados.

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

### Decisões e verificações necessárias para o conjunto final

O mantenedor precisa definir quais dos seguintes itens são efetivamente
candidatos: fonte Go, workspace Rust completo ou crates independentes, binários,
exemplos e fixtures; para binários ou crates, também os targets e features.
Esta escolha é separada de qualquer autorização para publicar ou distribuir.

Após essa definição, as verificações dependentes incluem listar e verificar os
arquivos de cada pacote extraído sem checkout adjacente; resolver dependências
locais e fixtures para crates independentes; gerar SBOMs do estado e alvo finais;
revisar os textos completos e avisos dos componentes efetivamente incluídos,
incluindo a questão de proveniência de `vsimd` quando aplicável; e obter a
aprovação apropriada para notices e distribuição. Binários exigem identificar
separadamente seus componentes nativos e características de build. Não se
infere conclusão desses itens dos testes ou inventários preliminares aqui.

### Comparação posterior com o checkout

No checkout em `22c8ba0`, conferiram-se os hashes dos arquivos registrados
nas cópias isoladas e compararam-se os nomes com `git ls-files --cached --others
--exclude-standard` para os mesmos escopos. Os 94 arquivos de `rust/` e
`conformance/fixtures/` da archive `f8313da0…b495131` e os 127 arquivos de
`go/` e fixtures da cópia Go coincidiram com o checkout; não havia nomes extras
ou ausentes nesses escopos. A comparação inclui alterações rastreadas e novos
arquivos não ignorados, não verifica arquivos ignorados e não abrange conteúdo
gerado, dependências do cache ou outros diretórios. Os documentos em
`docs/sbom/2026-11-05/` mudaram após a geração da archive Rust, portanto esta
comparação **não** afirma que a archive completa seja idêntica ao checkout atual.
Essas cópias continuam candidatas locais de investigação, sem aprovação de
notices ou distribuição.

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

Foi gerado um novo CycloneDX para o exemplo Rust extraído da archive
`f8313da0…b495131`, com `cargo-cyclonedx 0.5.9`, `--format json
--all-features --target x86_64-unknown-linux-gnu --spec-version 1.5` e
dependências locais offline. A archive do gerador teve SHA-256
`9bd3e599314f50810c9d98b8b68a617ff9d3cc20873968d90b29d121f6b226ff`,
igual ao valor anteriormente registrado. O resultado novo está na cópia
isolada, em `rust/examples/coordinator-telemetry/rust-telemetry-refresh-20261106.json`,
SHA-256 `375564f263a10bdceed2df7152f19dbe901d3269115b8171dec63d862fc85004`.
O JSON foi lido e informa CycloneDX 1.5 e 220 componentes; os pares
nome/versão coincidem com os 220 componentes do SBOM histórico. Essa
comparação não verifica relações, licenças, checksums ou schema CycloneDX;
a validação estrutural foi posteriormente realizada: a CLI CycloneDX 0.33.1,
binário SHA-256 `bfc8b2538da86fe239bc53658bbb63c1c8c510a293c1e6891aa5bea5d3c58746`,
confirmado contra o release upstream e a evidência anterior, retornou
`BOM validated successfully` para `validate --input-format json
--input-version v1_5 --fail-on-errors`. Isso não valida cobertura ou notices. O inventário não
inclui automaticamente dependências de teste nem comprova conteúdo de binário.
O SBOM original e sua quarentena não foram modificados. O arquivo gerado é
posterior à extração e não integra a archive original.

Comparando os 220 componentes por nome e versão com o SBOM histórico do exemplo
Rust, não houve diferenças nos campos `licenses` nem `evidence`; todos os
componentes novos têm licença ou evidência de licença. Esses campos não são
prova da correção da declaração nem abrangem o componente raiz ou todos os
avisos embutidos. A revisão anterior dos textos upstream continua aplicável
apenas às versões e aos escopos que examinou.

Para conferir a cadeia de telemetria, o inventário novo declara Apache-2.0
para `prometheus 0.14.0` e MIT para `protobuf 3.7.2` e `protobuf-support 3.7.2`.
As archives dessas três dependências no cache foram novamente verificadas
contra o lockfile da extração, com hashes respectivamente
`3ca5326d8d0b950a9acd87e6a3f94745394f62e4dae1b1ee22b2bc0c394af43a`,
`d65a1d4ddae7d8b5de68153b48f6aa3bba8cb002b243dbdbc55a5afbc98f99f4` e
`3e36c2f31e0a47f9280fb347ef5e461ffcd2c52dd520d8e216b52f93b0b0d7d6`.
Esses valores coincidem com `telemetry-examples/upstream-notices-review.md`,
que registra os textos e limitações da triagem anterior. Continua necessária a
revisão dos avisos embutidos, dependências transitivas e composição efetiva de
cada artefato escolhido; nenhuma licença final ou distribuição foi aprovada.

Uma comparação textual adicional confrontou o campo `licenses` dos 220
componentes com `package.license` dos `Cargo.toml` normalizados presentes no
cache local do registry: 204 são idênticos; em nove, o gerador converteu o
separador antigo `/` em `OR` (`bytes-utils`, `fallible-iterator`,
`fallible-streaming-iterator`, `ff`, `fnv`, `group`, `vcpkg`, `version_check`
e `xmlparser`). Os outros sete são crates locais `sqlite-replication-*`, com
PURLs `file://`, não entradas desse registry. A comparação é textual: não lê
os arquivos de licença, não verifica notices embutidos, nem garante que a
normalização corresponda a uma interpretação jurídica ou à distribuição
efetiva. Os caminhos locais dos PURLs identificam a extração de investigação,
não um pacote publicado. A revisão de textos e dos artefatos finais permanece
pendente.

Nas nove crates acima, as archives `.crate` do cache foram conferidas contra os
checksums do `Cargo.lock` do exemplo extraído: todas coincidiram. Uma busca em
todos os membros regulares por basename iniciado por `LICENSE`, `NOTICE` ou
`COPYING` encontrou `LICENSE-MIT` e `LICENSE-APACHE` em cada archive e nenhum
outro nome nesse conjunto. As versões são `bytes-utils 0.1.4`,
`fallible-iterator 0.3.0`, `fallible-streaming-iterator 0.1.9`, `ff 0.14.0`,
`fnv 1.0.7`, `group 0.14.0`, `vcpkg 0.2.15`, `version_check 0.9.5` e
`xmlparser 0.13.6`. Essa inspeção não lê integralmente os 18 textos, nem cobre
avisos embutidos ou com outros nomes. Uma busca por linhas de copyright nos
textos MIT encontrou créditos nas oito archives exceto `group`, cujo arquivo
MIT não apresenta linha explícita de titular de copyright; verificar os fontes
e a proveniência antes de preparar atribuições. A presença dos dois arquivos
não seleciona alternativa de licença nem comprova inclusão em um binário.

Posteriormente os nove arquivos `LICENSE-MIT` foram lidos integralmente de
suas archives verificadas contra o lockfile:

| Crate | SHA-256 do `LICENSE-MIT` | Copyright indicado no texto |
|---|---|---|
| `bytes-utils 0.1.4` | `ff3f1cd12af8866d9bde961d6cc40df774cd131d484de31d3170c4b02b21a7b5` | 2017 arc-swap developers |
| `fallible-iterator 0.3.0` | `0816e154b159ba255c563f7c8c7df5bbb8cc5fc96f5ab8cf9f4743b4f41fe7eb` | 2015 The rust-openssl-verify Developers |
| `fallible-streaming-iterator 0.1.9` | `8dcec5569a9be5b0e086c80faed6f1aefa670af0ec29cecc2f714303096887e0` | 2016 The fallible-streaming-iterator Developers |
| `ff 0.14.0` | `8d0c1d1d4b2bebf5b9211f0e883d5a482b7808f3ed31320da14ddee95811f4ef` | 2017 Sean Bowe |
| `fnv 1.0.7` | `65fdb6c76cd61612070c066eec9ecdb30ee74fb27859d0d9af58b9f499fd0c3e` | 2017 Contributors |
| `group 0.14.0` | `23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3` | Nenhum titular explícito neste arquivo; ver `COPYRIGHT` abaixo. |
| `vcpkg 0.2.15` | `016d20f335060a70e79d9fcf8dfaa6201114d65d592211f4bdfb8ae9ca2bc1dc` | 2017 Jim McGrath |
| `version_check 0.9.5` | `b7e650f3fce5c53249d1cdc608b54df156a97edd636cf9d23498d0cfe7aec63e` | 2017–2018 Sergio Benitez |
| `xmlparser 0.13.6` | `0b5f24524360e15bcf9fb79bca875f5aa62f01f94f2988e23238c7ec0e6e8784` | 2018 Reizner Evgeniy |

Os nove textos incluem condição de inclusão do aviso de copyright e permissão
em cópias ou porções substanciais e disclaimer; a tabela não substitui os
textos completos. Suas alternativas Apache, avisos em outros arquivos e
aplicabilidade ao artefato escolhido continuam por revisar. Os componentes
são dependências do grafo, não fontes vendorizadas na archive candidata.

Os nove arquivos `LICENSE-APACHE` também foram conferidos contra as archives
do cache, que correspondem ao lockfile. Os nove arquivos formam quatro grupos
de bytes idênticos; um texto de cada grupo foi lido integralmente:

| Crates | SHA-256 do `LICENSE-APACHE` |
|---|---|
| `bytes-utils 0.1.4`, `fnv 1.0.7`, `group 0.14.0`, `version_check 0.9.5`, `xmlparser 0.13.6` | `a60eea817514531668d7e00765731449fe14d059d3249e0bc93b36de45f759f2` |
| `fallible-iterator 0.3.0`, `fallible-streaming-iterator 0.1.9` | `c6596eb7be8581c18be736c846fb9173b69eccf6ef94c5135893ec56bd92ba08` |
| `ff 0.14.0` | `3708458dee7f359ac6c9c5558023ed481be87e5372c43ebd7f2ca7ad23c12026` |
| `vcpkg 0.2.15` | `60c93a31f490375aadf64098b75f10715010379541021e00261045d7800611d3` |

Os textos contêm Apache 2.0, incluindo condições de redistribuição e avisos
aplicáveis; seus apêndices apresentam campos exemplificativos, não atribuições
preenchidas. A identificação por hash não revisa avisos em fontes individuais,
não seleciona a alternativa Apache e não determina a composição de um binário.
Revisar tais avisos e a aplicabilidade dos textos ao artefato final permanece
necessário; esta tabela não substitui as licenças completas.

Busca textual nas nove archives verificadas pelo lockfile examinou 38 arquivos
`.rs` por `copyright`, `SPDX-License-Identifier`, `licensed under`,
`derived from`, `adapted from`, `copied from` e `based on`, sem distinção de
maiúsculas. As 16 linhas retornadas usam as últimas expressões em contexto
técnico, sem aviso explícito de autoria ou licença; por exemplo, o comentário
`based on` em `vcpkg/src/lib.rs` trata de compatibilidade de versões. A busca
não examina outros tipos de arquivo, outros idiomas ou avisos expressos de
outra forma; não comprova autoria integral, cobertura dos notices ou conteúdo
de binários. A inclusão dessas dependências em cada artefato e os avisos
aplicáveis continuam a depender da composição e configuração final escolhida.

Uma busca complementar nas mesmas nove archives verificadas examinou 893
arquivos não `.rs` decodificáveis como UTF-8, excluindo arquivos cujos basenames
começam por `LICENSE`, `LICENCE`, `COPYRIGHT`, `NOTICE` ou `COPYING`, que são
tratados separadamente. Os termos foram `copyright`, `SPDX-License-Identifier`,
`licensed under`, `derived from`, `adapted from` e `copied from`. Além de sete
linhas em READMEs que descrevem alternativas de licença, houve 285 linhas em
`vcpkg 0.2.15/test-data/`, principalmente listas que citam caminhos de arquivos
`copyright` de pacotes externos. Nenhum arquivo regular com basename exatamente
`copyright` foi encontrado na própria archive de `vcpkg`; os caminhos listados
não são, por si, prova de que seus conteúdos estejam incluídos. A revisão não
resolve todos os avisos possíveis nem determina que dados de teste ou esses
pacotes externos integrem qualquer artefato final. A composição de fontes,
testes e ferramentas de build deve ser decidida antes da revisão final.

Na busca posterior por basename `COPYRIGHT*` nessas nove archives, somente
`group 0.14.0` retornou resultado: `group-0.14.0/COPYRIGHT`, SHA-256
`7429802d3e67e3308471b91fbea341e484c4e0a23d0fb230a8636c38184cec79`.
O arquivo completo foi lido na cópia extraída do cache: informa que os direitos
permanecem com os contribuidores e apresenta Apache-2.0 ou MIT como alternativas.
A archive corresponde ao lockfile e o hash do arquivo extraído coincide com o
do membro da archive. Esse aviso é um exemplo que a
busca anterior, restrita a `LICENSE*`, `NOTICE*` e `COPYING*`, não identificava;
outros nomes ou avisos embutidos ainda podem existir. Os contribuidores não
foram individualmente identificados e a aplicabilidade a cada artefato segue
pendente.

#### Busca ampliada de avisos `COPYRIGHT` no exemplo Rust

Uma busca adicional nas 213 archives de registry vinculadas ao lockfile do
SBOM Rust de telemetria não encontrou nenhum membro regular cujo basename
comece, sem distinção de maiúsculas, por `LICENSE`, `LICENCE`, `COPYING`,
`COPYRIGHT` ou `NOTICE` em apenas três delas: `base64-simd 0.8.0`,
`vsimd 0.8.0` e `opentelemetry 0.33.1`. A última já está examinada em
`telemetry-examples/upstream-notices-review.md`. Os manifests das duas primeiras
declaram MIT e apontam ao repositório `Nugine/simd`; os respectivos arquivos
`.cargo_vcs_info.json` nas archives indicam o commit
`d74c030d9dc4f3cae02146d1f497ff62726ef09a`. No
[LICENSE upstream desse commit](https://github.com/Nugine/simd/blob/d74c030d9dc4f3cae02146d1f497ff62726ef09a/LICENSE),
o texto MIT credita copyright 2021 Nugine; o SHA-256 do texto obtido foi
`71674605ec4c087fe9eb534e3e4f9e26eb2e4aabcd76a29fd156c6a844d44b3d`.
As archives de `base64-simd` e `vsimd` têm respectivamente SHA-256
`339abbe78e73178762e23bea9dfd08e697eb3f3301cd4be981c0f78ba5859195` e
`5c3082ca00d5a5ef149bb8b555a72ae84c9c59f7250f013ac822ac2e49b19c64`,
conferidos com o lockfile. A referência ao commit não comprova equivalência
integral de fontes nem exclui avisos embutidos ou com outros nomes. Essas
dependências não são vendorizadas na archive-fonte do candidato; textos e
atribuições para eventual binário ou fonte distribuído requerem revisão por
artefato. Não houve escolha ou aprovação de licença para distribuição.

Nas cópias locais de `base64-simd 0.8.0` e `vsimd 0.8.0`, os 15 e 26
arquivos regulares respectivamente coincidem byte a byte com suas archives,
cujos checksums correspondem ao lockfile. Uma busca insensível a maiúsculas por
`copyright`, `SPDX`, `license`/`licence`, `derived from`, `adapted from`,
`based on` e `copied from` retornou apenas os manifests e READMEs declarando
MIT; ambos os READMEs apontam a `../../LICENSE`, que não integra essas archives.
Busca adicional por URLs nos fontes encontrou referências à especificação
WHATWG em `base64-simd/src/forgiving.rs` e, em `vsimd`, referências técnicas,
incluindo `src/alsw.rs` que afirma ser inspirado por um gist externo. Essa
afirmação de inspiração não prova cópia nem resolve proveniência ou termos de
terceiros; verificar sua relevância e os avisos para qualquer artefato final.
As buscas são limitadas aos termos e arquivos examinados, não substituem
revisão integral dos fontes nem determinam o conteúdo de um binário.

O gist citado em `vsimd/src/alsw.rs` foi consultado na revisão
`5ce93a2bb7512cab656a2f57e7d5b22bd595068e`: a página apresentou um arquivo
`base64_sse3_test.cpp` publicado por `aqrit`. O conteúdo bruto consultado tem
SHA-256 `164c54fa6799caa7d6c6344adaa6cc0762874ea2d0581f2874b7a186eab4614e`;
nele não foi identificada declaração explícita de licença. O arquivo `alsw.rs`
usa a palavra "inspired"; o gist contém uma função `hash` com operações
semelhantes às descritas na implementação Rust. Isso não estabelece cópia,
direitos ou condições de distribuição, nem afirma que o gist inteiro esteja
incorporado. A proveniência e eventual necessidade de autorização ou atribuição
continuam pendentes de revisão pelo responsável pelos artefatos finais.

O exemplo Rust seleciona também `outref 0.5.2`, dependência de `base64-simd`.
O SBOM e o manifesto declaram MIT. A archive do registry tem SHA-256
`1a80800c0488c3a21695ea981a54918fbb37abf04f4d0720c453632255e2ff0e`,
igual ao lockfile; seus 14 membros regulares coincidem com os arquivos da cópia
local examinada. O único basename iniciado por `LICENSE`, `LICENCE`, `COPYING`,
`COPYRIGHT` ou `NOTICE` é `LICENSE`, SHA-256
`a00511478c83ebb464ef19d840f061c7444adc4636bb561900e75c5808cfc6cc`.
O texto MIT foi lido integralmente e credita copyright 2022 Nugine; contém
condição de preservação do aviso e da permissão e disclaimer. Uma busca por
marcadores de copyright, SPDX, licença e derivação nas cópias locais retornou
o LICENSE, os manifests e o README, não outros avisos explícitos sob os termos
consultados. Isso não exclui avisos redigidos de outro modo nem prova inclusão
de `outref` no artefato final. A dependência não integra a archive-fonte do
candidato; sua aplicabilidade e os textos finais dependem da composição escolhida.

Nos 220 componentes do SBOM, os sete crates locais foram excluídos desta
consulta. As 213 archives de registry restantes foram encontradas no cache,
e seus SHA-256 coincidiram com o lockfile extraído. Uma busca em membros
regulares por basename iniciado por `COPYRIGHT` encontrou seis arquivos:

| Crate | SHA-256 do membro `COPYRIGHT` |
|---|---|
| `group 0.14.0` | `7429802d3e67e3308471b91fbea341e484c4e0a23d0fb230a8636c38184cec79` |
| `linux-raw-sys 0.12.1` | `3290ae0fbc9ddb77d2239121d710f0bb9d31b3b4744e6d97fe01e652b4c1870b` |
| `rand_core 0.10.1` | `92b81db30f7ab6d693e0af5661a40d273ca1947e891123ed823924192c10cbc7` |
| `rustix 1.1.6` | `377c2e7c53250cc5905c0b0532d35973392af16ffb9596a41d99d202cf3617c9` |
| `utf8_iter 1.1.0` | `c30152c94a6d75e021adbc52b3a52470366a46edb917e17deae3259251af244c` |
| `utf_types 1.0.0` | `3210be7332b5bdf48eb24a945258b9f38616a2cceb0dfc06e3c3c7e9740475a0` |

Os seis textos foram lidos nas cópias extraídas do cache, cujos hashes
correspondem aos respectivos membros das archives. `linux-raw-sys` e `rustix`
informam direitos retidos pelos contribuidores e alertam para arquivos derivados
de `libstd`; `rand_core` aponta autoria no histórico de versões. Os avisos de
`utf8_iter` e `utf_types` creditam Mozilla Foundation e distinguem trechos de
teste designados como domínio público/CC0; `utf8_iter` também aponta adaptação
de código da biblioteca padrão Rust e sua revisão de origem. O aviso de `group`
está detalhado acima. Estes resumos não substituem textos nem examinam as
exceções em arquivos individuais. A busca não cobre outros nomes, fontes de
crates locais, dependências de desenvolvimento omitidas pelo SBOM, ou quais
objetos são vinculados em binários; aplicabilidade e revisão integral de notices
continuam pendentes.

#### Índice de avisos embutidos para revisão posterior

Nas 213 archives de registry do SBOM de telemetria Rust, conferidas contra o
lockfile extraído, uma triagem por linha procurou, sem distinção de maiúsculas,
`copyright` como palavra, `SPDX-License-Identifier`, `licensed under`,
`derived from`, `adapted from` e `copied from`. Foram examinados 9.764 arquivos
UTF-8 com extensões `.rs`, `.c`, `.h`, `.s`, `.cc`, `.cpp` ou `.inc` (extensão
comparada sem distinção de maiúsculas); 2.877 arquivos de 101 crates tiveram
correspondências. O índice registra crate, membro, SHA-256 do arquivo, categoria
e números de linha, não a interpretação dos resultados:
`~/git-worktrees/sqlite-replication/sbom-refresh-20261106-evidence/rust-embedded-notice-index.json`,
SHA-256 `1afaf4d4660be9b49b6e65e22a552b51e79872712dbffe8fd08039977ec23952`.

A triagem inicial pulou somente `libsqlite3-sys-0.39.1/sqlite3/sqlite3.c`, por
exceder 8 MiB. Ele foi examinado separadamente em streaming, após conferir a
archive contra o lockfile: o membro tem SHA-256
`0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52`, e foram
encontradas 144 linhas com `copyright` e 30 com as expressões de origem, sem
correspondências nos outros dois grupos. O índice separado
`sqlite-amalgamation-notice-index.json` no mesmo diretório tem SHA-256
`6d12e02f77a56285232f977cf8c17d6f0bac96ded6cfcd406d80267e32854ef7`.

Esses índices são triagem, não revisão completa: podem conter falsos positivos,
não cobrem outros padrões, tipos de arquivo, crates locais ou dependências de
teste ausentes do SBOM. Ainda é preciso revisar os trechos pertinentes e definir
quais fontes ou objetos efetivamente integram cada artefato final. A quantidade
de resultados não determina obrigações, segurança jurídica ou aprovação de
distribuição; os arquivos originais e candidatos foram preservados.

Na revisão das 30 linhas classificadas como possível origem em `sqlite3.c`,
várias se referem a cópias de valores ou a código interno SQLite, não a
atribuições. Uma ocorrência material, na linha 37619, informa que o algoritmo
`sqlite3Fp10Convert2` é adaptado de trabalho de Ross Cox e referencia
`https://github.com/rsc/fpfmt`. O `LICENSE` da branch `main` desse repositório,
consultado posteriormente, contém BSD de três condições com copyright 2009
The Go Authors; o SHA-256 do texto obtido foi
`2d36597f7117c38b006835ae7f537487207d8ec407aa9d9980794b2030cbc067`.
Essa branch é móvel e não foi identificado o commit consultado pelos autores
SQLite: a referência e a licença atual não demonstram a versão de origem ou
quais termos se aplicam à adaptação. É necessário revisar proveniência e textos
pertinentes para qualquer artefato que incorpore a amalgamação; não se conclui
que todos os 30 resultados sejam avisos de terceiros nem que a atribuição esteja
resolvida. O membro SQLite examinado está vinculado ao hash registrado acima.

As 144 linhas da categoria `copyright` do índice da amalgamação foram
conferidas contra o arquivo cujo hash está acima. Há apenas dois textos de
linha distintos: 143 ocorrências de `The author disclaims copyright to this
source code. In place of` e uma de `The author disclaims copyright to this
source code.`. O contexto da primeira no início de `sqliteInt.h` inclui a
bênção SQLite; a linha isolada aparece no cabeçalho `fts3_tokenizer.h`, dentro
de uma seção condicional FTS3. Essa contagem por palavra não analisa todos os
contextos ou os demais arquivos da crate, não elimina avisos sem a palavra
`copyright` e não resolve a referência a Ross Cox descrita acima. Tampouco
estabelece quais seções condicionais compõem um binário final ou substitui a
revisão dos textos completos e das obrigações aplicáveis por artefato.

Na archive `rustix 1.1.6` selecionada pelo exemplo Rust, foram conferidos os
cabeçalhos de três arquivos: `src/weak.rs` identifica derivação da biblioteca
padrão Rust na revisão `fd0cb0cdc21dd9c06025277d772108f8d42cb25f`;
`src/types.rs` identifica cópia de `rust-lang/libc` na revisão
`008f905256d27eb5dd416d0976678ee6c2855554`; e
`src/maybe_polyfill/no_std/mod.rs` declara que a subárvore deriva da biblioteca
padrão e é licenciada MIT ou Apache 2.0. O último arquivo descreve sua seleção
para builds sem `std`; essa afirmação não determina as features do consumidor.
Os três arquivos examinados no cache coincidem com os membros da archive, que
corresponde ao checksum do lockfile. A busca textual por declarações semelhantes
em `.rs` encontrou outros arquivos `rustix` e nenhuma correspondência nos
arquivos `.rs` de `linux-raw-sys`, sob os padrões consultados; isso não exclui
outras formulações ou tipos de arquivo. A composição de um binário e seus
avisos aplicáveis seguem por revisar.

No mesmo conjunto de 213 archives de registry do SBOM de telemetria Rust, uma
busca independente por membros regulares cujo basename comece por `NOTICE`,
sem distinção de maiúsculas, retornou **zero arquivos**; as archives foram
novamente conferidas contra os checksums do lockfile antes da busca. Isso não
exclui avisos embutidos, arquivos com outros nomes, dependências de teste
omitidas do SBOM ou notices presentes somente upstream. Em particular, a
triagem `s3-aws-lc-transitive-notices.md` identifica notices upstream AWS-LC
que não constam da archive `.crate`; ausência local não elimina a necessidade
de revisá-los para qualquer artefato que incorpore os componentes pertinentes.
Nenhuma conclusão de distribuição decorre dessa contagem.

Em `utf8_iter 1.1.0`, o cabeçalho de `src/indices.rs` identifica adaptação de
`CharIndices` da biblioteca padrão Rust na revisão
`ab32548539ec38a939c1b58599249f3b54130026` e reproduz trecho do respectivo
aviso de autoria e referências às licenças Apache-2.0 e MIT. O SHA-256 desse
arquivo é `80d354f207007420d90391a161c7bddec57f76db63a596ee0a4831db1c383216`;
seus bytes na cópia do registry coincidem com o membro da archive, cujo checksum
corresponde ao lockfile. Não há membros regulares sob `tests/` nessa archive;
isso não exclui testes embutidos em fontes ou com outras localizações. O
`COPYRIGHT` menciona domínio público/CC0 somente para trechos de teste assim
designados; não se estendeu essa declaração ao arquivo `indices.rs`. A presença
no grafo não prova sua inclusão em um binário distribuído, e a revisão de
atribuições continua dependente da composição do artefato escolhido.

Em `utf_types 1.0.0/src/utf8.rs`, um comentário anterior ao módulo
`#[cfg(test)]` declara dedicar ao domínio público os direitos sobre o código
de teste abaixo dele, referenciando CC0 1.0; não se aplica essa declaração ao
restante do arquivo. Os bytes examinados correspondem ao membro da archive,
que coincide com o checksum do lockfile; SHA-256 de `src/utf8.rs`:
`1beed1d7bb3e8f75125db543894a57c1773484001e8c82637a6cb3a8e6c02e2e`.
Uma busca textual por `public domain`, `CC0` e `dedicat` nas árvores extraídas
de `utf_types` e `utf8_iter` também encontrou referências nos respectivos
`COPYRIGHT` e em `utf_types/CONTRIBUTING.md`, mas nenhum trecho designado
adicional na árvore de `utf8_iter` por esses termos. Isso não exclui avisos
expressos de outra forma, nem prova inclusão de testes no artefato final.

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

#### Inventários novos do módulo principal Go

Na cópia isolada Go/fixtures, `cyclonedx-gomod 1.12.0` gerou com Go 1.27.3,
`linux/amd64`, `GOPROXY=off` e `GOSUMDB=off` os inventários `mod -json
-licenses -type library`, sem e com `-test`, em caminhos novos sob
`/tmp/opencode/cyclonedx-refresh-iPmrUZod/generated/`. O SHA-256 da archive
do gerador, conferido com o release upstream e o registro anterior, foi
`004b9f5cc595b797fb5423e2ae4c97bcf0f18c712ed2faee1640b09e5efd6d15`.

| Arquivo | Componentes | SHA-256 |
|---|---:|---|
| `go-main.json` | 22 | `d2e87197c3e1d99bd1ae7c82fd9e7d543635f65af87c84fb6577fc17c6f7a706` |
| `go-main-with-tests.json` | 29 | `881b655836ba99f781076b53de18d40379988ac19c33b31956729e84df07b281` |

Ambos passaram `cyclonedx validate --input-format json --input-version v1_6
--fail-on-errors` com a CLI verificada acima. O gerador avisou que não pôde
determinar a versão do módulo raiz porque a cópia não é um repositório Git e
que não detectou sua licença. Detecções de terceiros são evidências, não
conclusões jurídicas, e continuam sujeitas à quarentena documentada para
classificações controversas. Essas saídas temporárias não são SBOMs de release.

Na mesma configuração offline, a tentativa equivalente para o exemplo Go de
telemetria falhou em `go list -mod readonly -json -m all`; a reprodução direta
retornou `module lookup disabled by GOPROXY=off`. Dois arquivos de saída vazios
foram criados pela redireção e não são SBOMs. A repetição com
`GOPROXY=https://proxy.golang.org` e `GOSUMDB=sum.golang.org`, em novos caminhos,
teve sucesso e ambos passaram a validação CycloneDX 1.6 com a CLI verificada:

| Arquivo em `generated/` | Componentes | SHA-256 |
|---|---:|---|
| `go-telemetry-online.json` | 25 | `50d6022acdda0785d1745f713ba1eafd71a82db099c2a991fbea38ea0412b2fe` |
| `go-telemetry-online-with-tests.json` | 40 | `fceaedbfedf0df4fd95fac551dfaa32601277ebcd89011667c2ed17c00e9f981` |

O gerador avisou que não identificou versões e licenças dos módulos locais da
cópia sem Git. A geração online não comprova disponibilidade offline em outro
host. As evidências automáticas de licença requerem revisão, inclusive as
classificações controversas já colocadas em quarentena nas saídas históricas;
nenhuma saída nova foi aprovada para distribuição. Os SBOMs históricos e cópias
em quarentena não foram substituídos.

#### Comparação e quarentena das novas evidências Go

A comparação por `(group, name, version)` dos componentes novos com os SBOMs
históricos correspondentes encontrou conjuntos idênticos para o módulo principal
com e sem testes. Nos dois inventários de telemetria, a única diferença nesse
conjunto foi o módulo local `sqlite-replication`: o histórico usava a
pseudoversão `v0.0.0-20261105182955-f5c0984f0694`, enquanto a cópia sem Git
deixou a versão ausente. Essa comparação não verifica relações ou propriedades.

As evidências controversas reapareceram em `go-main.json`,
`go-main-with-tests.json` e `go-telemetry-online-with-tests.json`: o detector
indicou MIT para `modernc.org/libc v1.80.1` e Apache-2.0 para
`modernc.org/sqlite v1.61.1`. Os arquivos principais upstream têm textos BSD
de três condições, como detalhado em `license-detector-investigation.md`;
nenhuma dessas evidências isoladas cobre todos os textos de terceiros.
O inventário de telemetria sem testes não contém esses dois componentes.

O script `conformance/quarantine-sbom-license-evidence.py` criou três cópias
novas no mesmo diretório temporário; comparação estrutural confirmou que apenas
as duas entradas `evidence.licenses` foram removidas em cada uma. Todas passaram
`cyclonedx validate --input-format json --input-version v1_6 --fail-on-errors`:

| Cópia derivada | SHA-256 |
|---|---|
| `go-main-quarantined.json` | `b8462e49d3a7d3d8b129977237130a37fc5db2edf703198685318b2e9ad6b1f1` |
| `go-main-with-tests-quarantined.json` | `8ad55a2558533b74379de72e1d85bd5a3636695cacff3f656d7c8e16f5a38267` |
| `go-telemetry-online-with-tests-quarantined.json` | `e563e4fb723ca6acf87c259fff4968a80340a9802c2f7f18a419a1bc1b445fd7` |

Os originais novos e históricos foram preservados. Ausência de evidência não
é atribuição de licença, e a quarentena não aprova notices ou distribuição;
persistem outras evidências automáticas e a revisão por artefato final.

Para preservar a evidência além do diretório temporário, os quatro inventários
Go e as três cópias derivadas em quarentena listados acima foram copiados para
`~/git-worktrees/sqlite-replication/sbom-refresh-20261106-evidence/`, em um
diretório novo no filesystem persistente de `/home`. Antes e depois da cópia,
os sete SHA-256 foram conferidos com as tabelas acima; os arquivos não foram
normalizados nem alterados. As saídas temporárias e os SBOMs históricos foram
preservados. O SBOM Rust atualizado permanece na extração persistente do
candidato, no caminho indicado anteriormente. Esses diretórios são evidências
locais, não componentes de um pacote final nem autorização de distribuição.

Comparação adicional dos valores de `evidence.licenses` por nome de componente
comum aos quatro pares de inventários novo/histórico encontrou **zero diferenças**
em cada par. Isso inclui as duas classificações controversas acima e não prova
que as demais sejam corretas. Nos inventários novos não derivados, todos os
componentes do módulo principal têm alguma licença declarada ou evidência de
licença; nos dois inventários do exemplo, o componente da substituição local
`sqlite-replication` não tem nenhuma. A contagem não inclui o componente raiz
em `metadata.component`, não verifica conteúdo de licença ou avisos por arquivo
e não converte detecções em conclusões jurídicas. A revisão de textos principais
e avisos específicos está em `native-notices-review.md`, na triagem de telemetria
e nos demais documentos referenciados; identificar todos os avisos aplicáveis
a um artefato definido continua pendente.

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

Nas nove crates adicionais de desenvolvimento enumeradas acima, as archives do
cache coincidiram com os checksums do lockfile do exemplo. Busca em todos os
membros regulares por basename iniciado por `COPYRIGHT` encontrou um arquivo
em cada uma de `rand 0.9.5`, `rand_chacha 0.9.0` e `rand_core 0.9.5`; as outras
seis não tinham membros com esse prefixo. Os três arquivos são idênticos,
SHA-256 `90eb64f0279b0d9432accfa6023ff803bc4965212383697eee27a0f426d5f8d5`,
e coincidem com as cópias locais extraídas. O texto completo informa que os
direitos permanecem com contribuidores, remete autoria completa ao histórico
e identifica código do projeto Rust sob as mesmas alternativas MIT ou
Apache-2.0, ressalvadas exceções em arquivos individuais. Estas dependências
são adicionais na seleção com desenvolvimento; os candidatos-fonte não as
vendorizam. A busca por nome não cobre avisos embutidos, nem demonstra que
integrem um artefato final. A decisão de incluir fontes de teste, binários de
teste ou apenas artefatos de runtime permanece com o mantenedor e determina a
revisão de avisos correspondente.

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

A archive `lz4_flex 0.14.0` do cache tem SHA-256
`ecbdfe44b1bd960b68170b417450a628c43f7cf56bb3c5317e61cb230ee7f226`,
igual ao checksum do `Cargo.lock` do candidato `f8313da0…b495131`. Seu membro
`src/frame/header.rs`, SHA-256
`c9926623a51e20325e717df62bdf1ad0ca24b787e7e04e8122a38d8a28587cc4`, coincide
com a cópia do registry examinada em `ltx-transitive-notices.md`. Portanto a
investigação histórica ali registrada se refere aos mesmos bytes da dependência
fixada do candidato. O commit exato da especificação consultado pelo autor da
crate e as obrigações aplicáveis ao recorte continuam não determinados; essa
dependência não está vendorizada na archive-fonte de investigação.

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

No `fresh-target` criado para a extração e testes de `f8313da0…b495131`, a
archive intermediária `debug/build/aws-lc-sys-89ce0ebf005deffc/out/
libaws_lc_0_47_0_crypto.a` tem SHA-256
`9b51d35e7f0a590bd9d19193879c9cd69e102f29bb1329784f0270400c3952f3`,
idêntico ao intermediário examinado em `s3-aws-lc-transitive-notices.md`.
`ar t` confirmou membros com nomes `jitterentropy`, `mlkem`, `mldsa` e
`keccak` nessa archive. Assim, a revisão amostral de cabeçalhos e os textos
upstream ali registrados são evidência relevante para esse intermediário exato;
isso não prova que tais objetos sejam vinculados a um executável específico,
nem substitui revisão de outros componentes ou do artefato a distribuir.

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
