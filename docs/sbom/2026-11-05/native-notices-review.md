# Revisão técnica parcial: binários locais e notices nativos

Esta análise de 2026-11-05 não é parecer jurídico nem lista final de notices.
**Nenhum artefato foi autorizado para distribuição.** O escopo são os dois
executáveis locais de conformance usados no benchmark, não packages publicados
ou todas as configurações de produção. Os arquivos foram construídos a partir de
`f5c0984`, com árvore marcada como modificada; as alterações de então eram
documentação e teste, não código de produção. SHA-256 dos binários e toolchains
estão em `../../litestream-benchmark-feasibility.md`. O binário Litestream é
uma ferramenta externa para benchmark, não uma dependência declarada para
distribuição do projeto.

## Go: inventário do executável específico

### Seleção adicional do pacote `replicator` — checkout `a630d16`

Em Linux amd64, com Go 1.27.3, `GOPROXY=off` e `-mod=readonly`,
`go list -deps` para `./cmd/conformance` selecionou os mesmos nove módulos
externos listados abaixo. Para `./replicator`, selecionou também os seguintes
11 módulos AWS/Smithy; esta consulta é de pacotes selecionados para uma
configuração, não um link map nem uma lista de arquivos distribuídos:

| Módulo adicional | Versão |
|---|---|
| `github.com/aws/aws-sdk-go-v2` | `v1.47.6` |
| `github.com/aws/aws-sdk-go-v2/aws/protocol/eventstream` | `v1.7.26` |
| `github.com/aws/aws-sdk-go-v2/internal/configsources` | `v1.5.9` |
| `github.com/aws/aws-sdk-go-v2/internal/endpoints/v2` | `v2.8.9` |
| `github.com/aws/aws-sdk-go-v2/internal/v4a` | `v1.5.9` |
| `github.com/aws/aws-sdk-go-v2/service/internal/accept-encoding` | `v1.13.24` |
| `github.com/aws/aws-sdk-go-v2/service/internal/checksum` | `v1.11.10` |
| `github.com/aws/aws-sdk-go-v2/service/internal/presigned-url` | `v1.14.9` |
| `github.com/aws/aws-sdk-go-v2/service/internal/s3shared` | `v1.20.9` |
| `github.com/aws/aws-sdk-go-v2/service/s3` | `v1.114.7` |
| `github.com/aws/smithy-go` | `v1.28.8` |

Na raiz dos dez módulos AWS selecionados, os arquivos `LICENSE.txt` do cache
local têm SHA-256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`.
O módulo AWS principal também contém `NOTICE.txt`, SHA-256
`a6e830174d62dafad3a718384772ea1f4c0e27989f050932dc442a5e3ddad080`, com
créditos Amazon e Stripe. O `smithy-go` contém `LICENSE` Apache-2.0, SHA-256
`09e8a9bcec8067104652c168685ab0931e7868f9c8284b66f5ae6edae5f1130b`, e
`NOTICE`, SHA-256 `d4290ed64c2edd0fce1d84e3f9dfb2881240fe534def76b8cd29ed6af683e287`,
com crédito Amazon. A inspeção foi limitada aos nomes na raiz dos módulos;
não demonstra ausência de avisos aninhados, composição de binários, integridade
dos caches frente aos arquivos upstream ou obrigações jurídicas finais. Os
textos completos aplicáveis ainda devem ser revistos para cada artefato escolhido.

`go version -m` do executável Go lista nove módulos de terceiros: `go-humanize`
1.1.0, `google/uuid` 1.6.0, `pierrec/lz4/v4` 4.1.22, `bigfft` na revisão
`24d4a6f8daec`, `golang.org/x/sys` 0.49.0, `modernc.org/libc` 1.80.1,
`modernc.org/mathutil` 1.7.1, `modernc.org/memory` 1.12.1 e
`modernc.org/sqlite` 1.61.1. Isso é a lista de módulos informada pelo build,
não prova de que cada arquivo desses módulos esteja incorporado. O Go runtime
e código nativo/sistema também exigem revisão. Outros executáveis, em especial
os que usam S3, terão outra composição.

### Pacotes selecionados na configuração atual

Uma consulta posterior `go list -mod=readonly -deps` para `./cmd/conformance`,
com Go 1.27.3, `GOOS=linux`, `GOARCH=amd64` e `GOPROXY=off`, identificou os
seguintes pacotes de terceiros. As licenças na tabela são os textos principais
examinados no cache, não classificações completas dos componentes incorporados.

| Módulo selecionado | Pacotes selecionados | Texto principal e ressalvas |
|---|---|---|
| `golang.org/x/sys v0.49.0` | `unix` | BSD de três condições; revisar seus avisos e arquivos selecionados. |
| `github.com/dustin/go-humanize v1.1.0` | raiz | MIT; preservar texto e copyright aplicáveis. |
| `github.com/google/uuid v1.6.0` | raiz | BSD de três condições. |
| `github.com/remyoudompheng/bigfft v0.0.0-20230129092748-24d4a6f8daec` | raiz | BSD de três condições. |
| `modernc.org/mathutil v1.7.1` | raiz | BSD de três condições; revisar avisos por arquivo. |
| `modernc.org/memory v1.12.1` | raiz | BSD de três condições e avisos Go/mmap distintos, detalhados abaixo. |
| `modernc.org/libc v1.80.1` | raiz, `sys/types`, `uuid/uuid` | BSD de três condições mais avisos de terceiros, detalhados abaixo. |
| `modernc.org/sqlite v1.61.1` | raiz, `lib`, `vtab` | BSD de três condições mais SQLite e notices transitivos; não se reduz à classificação automática do SBOM. |
| `github.com/pierrec/lz4/v4 v4.1.22` | raiz e `internal/lz4errors`, `lz4block`, `xxh32`, `lz4stream` | BSD de três condições no arquivo principal; consultar também `ltx-transitive-notices.md` para histórico de notices. |

A consulta também selecionou pacotes do próprio projeto e biblioteca padrão Go;
estes não estão representados pela tabela de terceiros. A seleção atual não
identificou os pacotes opcionais `modernc.org/sqlite/vec` ou `sqlite/vfs`, mas
isso não verifica outros targets, tags ou artefatos. A lista concorda em nomes e
versões de módulos externos com o inventário anterior de `go version -m` do
binário histórico; **não** é um link map nem prova que todos os arquivos e
símbolos selecionados foram conservados nesse binário. Não foi refeito o build
histórico nem definida distribuição, e a revisão legal de todos os textos e
fontes aplicáveis continua pendente.

### `github.com/remyoudompheng/bigfft`: fontes e avisos

Na configuração atual Linux amd64 com Go 1.27.3 offline, `go list` selecionou
`arith_decl.go`, `fermat.go`, `fft.go` e `scan.go` do módulo fixado
`v0.0.0-20230129092748-24d4a6f8daec`; imports declarados: `math/big` e
`unsafe`. `modernc.org/mathutil@v1.7.1/sqr.go` importa `bigfft` e chama
`bigfft.Mul`, explicando uma ligação no grafo de pacotes, não necessariamente
uma chamada executada ou função retida no binário histórico.

O `LICENSE` do módulo credita The Go Authors, copyright 2012, e contém BSD de
três condições com aviso de preservação para redistribuições. `arith_decl.go`
credita The Go Authors, copyright 2010, e contém diretivas `go:linkname` para
rotinas internas de `math/big`; `fermat.go` identifica `basicMul` como
`copied from math/big`. Esses avisos evidenciam origem e dependência declaradas,
não identificam todas as contribuições ou provam quais funções entram no
binário. O README upstream chama a biblioteca de prova de conceito e recomenda
avaliar sua necessidade: essa é uma advertência do mantenedor, não evidência
de falha ou vulnerabilidade nesta aplicação.

O binário de conformance anteriormente inventariado lista o módulo, mas a
seleção atual de arquivos não é link map do build histórico e não demonstra
execução de operações FFT. Preservar e revisar textos completos Go e bigfft,
autoria das contribuições e composição real por artefato ou source bundle;
nenhuma conclusão jurídica, modificação de SBOM ou publicação decorre desta
inspeção.

### LZ4 Go e implementação interna xxHash

Na configuração atual Linux amd64 com Go 1.27.3 offline, `go list` selecionou
oito arquivos Go no pacote raiz `github.com/pierrec/lz4/v4 v4.1.22` e dois em
`internal/xxh32`: `xxh32zero.go` e `xxh32zero_other.go`; nenhum assembly foi
selecionado nesses dois pacotes. O `LICENSE` principal é BSD de três condições,
copyright Pierre Curto, e sua cláusula de não endosso menciona `xxHash`.
O texto completo e disclaimer devem ser revistos, não apenas o identificador.

`internal/xxh32/xxh32zero.go` afirma ser port da implementação de referência
xxHash e aponta ao repositório upstream; o commit
[`edc95a188bc81afa4a34b114aafcb8d41513dc84`](https://github.com/pierrec/lz4/commit/edc95a188bc81afa4a34b114aafcb8d41513dc84)
de 2022 alterou o comentário para explicitar esse port, sem fixar revisão de
origem. Como referência, o [`LICENSE` de xxHash v0.8.2](https://github.com/Cyan4973/xxHash/blob/v0.8.2/LICENSE)
declara BSD de duas condições e copyright Yann Collet. Essa versão é apenas
uma amostra upstream, **não** a versão comprovada de origem do port; a
aplicabilidade e os avisos necessários precisam de revisão por arquivo e por
artefato. A comparação histórica da especificação LZ4 em
`ltx-transitive-notices.md` refere-se ao crate Rust `lz4_flex`, não prova
origem dos fontes Go aqui examinados.

O módulo LZ4 consta no inventário do binário de conformance histórico, mas
seleção atual não comprova a retenção de cada arquivo ou função no executável.
Builds com outras tags ou arquiteturas e pacotes de fontes podem conter
componentes adicionais. Não houve alteração de SBOM ou publicação; preservar
os avisos e submeter composição e direitos a revisão antes da distribuição.

#### Histórico do port `xxh32` Go

O histórico do caminho localiza `internal/xxh32/xxh32zero.go` no commit
[`7f4bd64aba4b03e434003ac9e586d71d444527f8`](https://github.com/pierrec/lz4/commit/7f4bd64aba4b03e434003ac9e586d71d444527f8),
de 2018-06-09, descrito como grande refatoração. O diff adiciona tanto
`xxh32.go` quanto `xxh32zero.go`; o comentário inicial apontava para
`Cyan4973/XXH`, sem fixar revisão. O teste adicionado nesse commit importava
`github.com/pierrec/xxHash/xxHash32`, mas isso é evidência de dependência de
teste, não prova de que a implementação de produção foi copiada desse módulo.
Em 2022 o comentário foi alterado para declarar port da referência xxHash,
conforme o commit já citado. Esse histórico não demonstra o primeiro momento
de criação do algoritmo ou a revisão exata consultada.

Uma consulta ao histórico do caminho `LICENSE` no repositório xxHash, até a
data de autoria do commit de 2018, retornou como alteração mais recente
[`f0f007f0424c12997a33d604c01664b30bb248f4`](https://github.com/Cyan4973/xxHash/blob/f0f007f0424c12997a33d604c01664b30bb248f4/LICENSE)
de 2014. O texto então disponível declara copyright Yann Collet 2012–2014 e
duas condições de redistribuição, além de disclaimer. A consulta usa o histórico
da branch padrão atual e não prova o estado da branch ou revisão efetivamente
usada pelo autor do port. Os avisos de Pierre Curto no módulo LZ4 e os possíveis
avisos da origem xxHash precisam ser revisados juntos, nos textos completos e
para cada artefato, sem declarar automaticamente a combinação jurídica.

Não foi comparado integralmente o port com uma revisão histórica da referência,
nem identificadas as funções vinculadas no executável final. A investigação é
documental e não altera SBOMs, contrato ou autorização de distribuição.

#### Comparação amostral com xxHash histórico

A API GitHub retornou o commit
[`5d41eb818482bc08ab05f7049313836b906b4bdc`](https://github.com/Cyan4973/xxHash/blob/5d41eb818482bc08ab05f7049313836b906b4bdc/xxhash.c),
de 2018-03-20, como última alteração de `xxhash.c` na branch padrão até a
data de autoria do commit LZ4 Go de 2018. Os cinco valores das constantes
XXH32 examinados coincidem com os de `internal/xxh32/xxh32.go` introduzido
naquele commit. Na comparação amostral, `xxh32zero.go` usa operações de rodada
com multiplicação pela constante 2, rotação 13 e multiplicação pela constante
1, e finalização com deslocamentos 15/13/16 e multiplicações pelas constantes
2/3, também presentes nas funções `XXH32_round` e `XXH32_avalanche` do C.
Os dois fontes têm linguagens e organização diferentes; não foram comparados
integralmente nem executados testes de equivalência. Semelhanças de constantes
e algoritmo são compatíveis com o port declarado, mas não identificam a revisão
efetivamente usada ou provam cópia textual.

Nessa amostra, `xxhash.c` e `xxhash.h` contêm cabeçalhos BSD de duas condições
com copyright Yann Collet **2012–2016**, enquanto o `LICENSE` da mesma árvore
exibe **2012–2014**. Preservar a diferença nos avisos, não substituir
automaticamente um pelo outro. Os textos do upstream e o BSD de três condições
do módulo LZ4 Go devem ser revisados por arquivo e artefato, inclusive para
source bundles. Essa comparação documental não comprova direitos, seleção no
executável ou equivalência funcional e não autoriza publicação.

#### Comparação por etapas XXH32

Foi inspecionado o `xxh32zero.go` do commit LZ4 Go de 2018 e o `xxhash.c`
da revisão histórica C indicada acima, ambos obtidos por HTTP 200. A tabela
compara as etapas visíveis do caminho **one-shot** `ChecksumZero` com
`XXH32_endian_align`/`XXH32_finalize` para seed zero; não é comparação
exaustiva de todas as entradas ou do caminho streaming:

| Etapa | Correspondência textual observada |
|---|---|
| Inicialização | Para entradas de pelo menos 16 bytes, quatro acumuladores com constantes 1+2, 2, zero e negativo de 1; para entradas curtas, constante 5. |
| Blocos | Grupos de 16 bytes em quatro palavras de 32 bits; rodada com constante 2, rotação 13 e constante 1. O Go lê explicitamente little-endian; o C usa helper com opções de alinhamento e endian. |
| Combinação | Soma das rotações 1, 7, 12 e 18 dos acumuladores, mais comprimento convertido a 32 bits. |
| Cauda | Palavras de quatro bytes usam constantes 3/4 e rotação 17; bytes restantes usam constantes 5/1 e rotação 11. O Go usa loops; o C usa `switch` com fallthrough. |
| Avalanche | XOR com deslocamentos 15, 13 e 16, intercalados com multiplicações pelas constantes 2 e 3. |

O port adicionalmente implementa `Write`, `Sum32` e `Uint32Zero`; os caminhos
de estado, limites, seed, alinhamento e mudanças posteriores à introdução não
foram comparados exaustivamente. A estrutura coerente com XXH32 não prova
equivalência, ausência de falhas, autoria ou revisão exata de origem. Os
avisos diferentes de copyright 2012–2014/2012–2016 e licenças completas estão
registrados acima; sua aplicabilidade depende dos arquivos e artefatos
efetivamente distribuídos e de revisão jurídica. Nenhum teste funcional ou
build foi executado nesta comparação e nenhum SBOM foi alterado.

#### Caracterização do caminho streaming XXH32

Foi comparado `XXHZero` do cache `github.com/pierrec/lz4/v4@v4.1.22` com
`XXH32_update_endian`/`XXH32_digest_endian` da revisão C histórica
`5d41eb818482bc08ab05f7049313836b906b4bdc`. Ambos acumulam bytes até 16,
processam quatro palavras, guardam a cauda e finalizam com mistura semelhante;
o Go usa seed fixo zero e `updateGo`, enquanto o C expõe seed e status próprios.
Essa leitura não prova equivalência para todas as entradas ou divisão de writes.

Em uma cópia temporária isolada dos dois arquivos Go selecionados para amd64,
foi executado com Go 1.27.3 offline um teste de caracterização de duas escritas
de oito bytes. Resultado: primeira `Write` retornou `(8, nil)`, segunda
`(0, nil)`, embora o `Sum32` resultante tenha coincidido com o `ChecksumZero`
dos 16 bytes (`78ac7056`). O teste **passou porque registra esse comportamento**,
não porque o retorno siga o contrato convencional de `io.Writer`. Na fonte,
ao completar buffer anterior, `n` é reduzido antes de retornar. Na versão LZ4
examinada, três chamadas de checksum em `internal/lz4stream/block.go` descartam
explicitamente contagem e erro; não se conclui impacto para todos os chamadores
ou builds a partir dessas ocorrências. Não se modificou o cache nem a dependência.

Outra diferença identificada estaticamente: o Go converte `totalLen` para `uint32`
antes de decidir `>= 16` em `Sum32`; o C histórico mantém `large_len` separado
e monotônico. O ensaio direcionado posterior abaixo confirmou divergência em
alguns comprimentos após wrap; não se afirma corrupção nesta aplicação sem
reproduzir um fluxo real.
Nem o resultado pontual nem essa hipótese demonstram equivalência com a
referência C, falha de conformance do projeto, ou comportamento do executável
histórico. Os avisos e revisão de direitos acima continuam independentes dessa
caracterização; não houve alteração de contrato, SBOM ou publicação.

#### Ensaio isolado na fronteira de 4 GiB

Em Linux x86-64, com Go 1.27.3 e dependência offline, uma cópia temporária dos
dois arquivos Go selecionados de `xxh32` processou um fluxo sintético de zeros
em chunks de 1 MiB. Em processo separado, `libxxhash.so.0` do host, consultada
por `ctypes` e retornando `XXH_versionNumber() = 804`, processou os mesmos
chunks com seed zero. O teste Go é de caracterização e não faz asserção de
igualdade com a referência; sua saída `PASS` significa apenas que terminou.
Sem alocar ou gravar 4 GiB, foram obtidos:

| Comprimento | Go streaming | libxxhash streaming | Resultado |
|---|---|---|---|
| 2³² − 1 | `10412e64` | `10412e64` | igual |
| 2³² | `02cc5d05` | `35b93941` | diferente |
| 2³² + 1 | `cf65b03e` | `edd46a0b` | diferente |
| 2³² + 16 | `904778da` | `904778da` | igual |

O resultado confirma divergência nesses dois pontos para esta sequência e
implementações; não demonstra equivalência nos demais comprimentos ou versão
histórica C específica. A conversão do comprimento para `uint32` antes da
decisão `>= 16` é consistente com a janela observada, não é uma análise
exaustiva. O teste também ignorou o retorno de contagem de `Write` já
caracterizado acima. Não se modificou o código da dependência.

No checkout Go, `ltx/format.go` cria um novo `lz4.NewWriter` por página tanto
para snapshot quanto delta e valida tamanhos SQLite entre 512 e 65.536 bytes;
portanto a **entrada normal de cada frame gerado nesses caminhos** é de uma
página, não de 4 GiB. O leitor também cria `lz4.NewReader` por página, lê uma
página e rejeita um byte extra. Isso não prova segurança completa diante de
frames malformados: o decoder LZ4 possui processamento assíncrono de blocos e
checksum interno, cuja pré-leitura e limites não foram caracterizados aqui.
Não foram examinados outros consumidores do módulo ou fluxos Rust. Não se
afirma impacto reproduzido no projeto, nem compatibilidade universal, com base
no teste sintético. Investigar limites e cancelamento do decoder antes de
concluir ausência de exposição; o gate e contrato de admissão seguem abertos.

#### Pré-leitura de frame maior que uma página

Em ambiente temporário no host Linux x86-64, Go 1.27.3, `GOPROXY=off` e
`GOMAXPROCS=2`, um programa de caracterização gerou dados pseudoaleatórios
determinísticos em memória, codificou um frame LZ4 válido de conteúdo maior
que uma página LTX de 4.096 bytes e repetiu o padrão do leitor do projeto:
`io.ReadFull` de 4.096 bytes, seguido de leitura de um byte e rejeição caso
esta não retorne EOF. O input é LZ4 válido, porém inválido **como frame de uma
página LTX**; não foi construído um arquivo LTX completo. Um wrapper contou os
bytes consumidos do frame, não memória alocada ou bytes hasheados:

| Bloco configurado | Conteúdo | Checksum de conteúdo | Bytes do frame | Consumidos após página e byte extra | Resultado da regra LTX |
|---|---:|---|---:|---:|---|
| 64 KiB | 2 MiB | desligado | 2.097.291 | 65.547 | rejeita: byte extra |
| 64 KiB | 2 MiB | ligado | 2.097.295 | 65.547 | rejeita: byte extra |
| 4 MiB | 4 MiB | desligado | 4.194.319 | 4.194.315 | rejeita: byte extra |
| 4 MiB | 4 MiB | ligado | 4.194.323 | 4.194.315 | rejeita: byte extra |

O construtor `lz4.NewReader` dessa versão aplica por padrão
`ConcurrencyOption(1)`; o programa não mudou a concorrência do leitor. Portanto
estes resultados **não** caracterizam modo concorrente opcional, todos os tipos
de frame, entradas truncadas, memória máxima, validação do checksum do frame
completo, ou goroutines após descarte. O código de `lz4stream/block.go` atualiza
o checksum de conteúdo por bloco quando habilitado, mas o teste não instrumentou
quantos bytes foram hasheados nem chegou a 4 GiB. O resultado mostra
pré-leitura de bloco antes da rejeição, inclusive 4 MiB no caso testado, mas não
prova que o caso de wrap XXH32 seja alcançável através do decoder do projeto.
Outras configurações e entradas exigem testes e análise de limites próprios;
não houve alteração da dependência, contrato ou publicação.

#### Decoder de delta LTX com frame sobredimensionado

Em `/tmp/opencode/ltx-oversize-ltjbl427`, uma cópia de `go/ltx/format.go`
e uma cópia local de LZ4 4.1.22 foram usadas com Go 1.27.3, `GOPROXY=off`
e `GOMAXPROCS=2`. Apenas na cópia, `XXHZero.Write` foi instrumentado para
imprimir o tamanho de cada chamada. Um delta controle produzido por
`EncodeDelta` foi decodificado sem erro. Depois, um delta sintético com
cabeçalho, terminador, índice e trailer foi construído com frame LZ4 de
conteúdo 4 MiB para uma página declarada de 4.096 bytes. A estrutura foi
montada para o ensaio, mas o frame é inválido para a página e o decoder o
rejeita antes de se comprovar a validade do índice ou CRC; não é um LTX
semanticamente válido.

Após o marcador `BEGIN_DECODER`, a instrumentação registrou uma chamada
`XXH_INSTRUMENT bytes 4194304`. O `DecodeDelta` retornou erro associado a
`ErrInvalidFile` para a página 1; o contador da fonte registrou 4.194.419
bytes lidos de um arquivo de 4.194.462 bytes, cujo frame tinha 4.194.323
bytes. A contagem inclui cabeçalho LTX e possível buffering: não é posição
lógica nem medida de memória. A chamada instrumentada mede bytes entregues
ao checksum, não verifica que o frame inteiro ou trailer tenha sido validado.
O ensaio evidencia que um bloco descomprimido de 4 MiB pode ser hasheado
antes da rejeição, mas não alcançou 4 GiB e não prova que a divergência após
wrap seja alcançável no decoder ou ausente em outras entradas. Nenhuma
dependência do cache, fixture ou contrato foi alterado.

### `github.com/google/uuid v1.6.0`: arquivos e aviso

Com Go 1.27.3, Linux amd64 e `GOPROXY=off`, `go list -mod=readonly`
selecionou 15 arquivos do pacote: `dce.go`, `doc.go`, `hash.go`, `marshal.go`,
`node.go`, `node_net.go`, `null.go`, `sql.go`, `time.go`, `util.go`,
`uuid.go`, `version1.go`, `version4.go`, `version6.go` e `version7.go`;
`node_js.go` foi ignorado. Seus imports reportados são da biblioteca padrão Go.
Uma busca nos primeiros cinco versos encontrou em **15/15** a expressão de
copyright Google Inc. e referência ao `LICENSE`.

O `LICENSE` principal credita Google Inc. e contém BSD de três condições,
incluindo preservação de copyright, condições e disclaimer para redistribuição
de fontes ou em materiais de binários, conforme o texto. Isso não substitui
revisão dos arquivos completos, do runtime Go ou de outros componentes.
O binário de conformance já inventariado lista o módulo v1.6.0; a seleção atual
não é link map nem comprova que cada função ou arquivo tenha permanecido no
executável histórico. Outras plataformas, tags, testes e source bundles podem
incluir fontes diferentes. Não se alteraram SBOMs ou artefatos, e a revisão
jurídica por distribuição permanece pendente.

### `github.com/dustin/go-humanize v1.1.0`: fonte adaptada

`go list -mod=readonly` com Go 1.27.3, Linux amd64 e `GOPROXY=off`
selecionou 11 arquivos do pacote: `big.go`, `bigbytes.go`, `bytes.go`,
`comma.go`, `commaf.go`, `ftoa.go`, `humanize.go`, `number.go`, `ordinals.go`,
`si.go` e `times.go`. Os imports reportados são da biblioteca padrão Go. O
`LICENSE` do módulo contém texto MIT com copyright Dustin Sallings; uma busca
textual nos arquivos Go encontrou, em `number.go`, indicação de adaptação de
[gist externo de gorhill](https://gist.github.com/gorhill/5285193), com crédito
ao autor e link à fonte.

A API GitHub do gist, consultada em sua revisão atual
[`cc36e402754549e658cf1d7255fd84f3c284f063`](https://gist.github.com/gorhill/5285193/cc36e402754549e658cf1d7255fd84f3c284f063),
retornou o texto `Feel free to reuse as you wish` no código e um arquivo
`~LICENSE` contendo WTFPL versão 2. O histórico da API atribui essa revisão
a 2013-09-15. Não foi estabelecida a revisão exata originalmente adaptada, a
autoria de todo o material ou a aplicabilidade dos termos do gist à cópia e
a suas modificações; não se substitui automaticamente o MIT do módulo nem se
assume que ele resolva todos os direitos. Consultar os textos completos e
submeter a proveniência à revisão de direitos antes de distribuir fontes ou
outros artefatos pertinentes.

O módulo consta no inventário do binário Go histórico; a seleção atual de
`number.go` não comprova que suas funções tenham sido retidas ou executadas
nesse executável. Uma distribuição de fontes pode ter escopo diferente. Não
se alteraram SBOMs ou contrato e não houve publicação; falta revisão por
artefato, incluindo código, biblioteca padrão e notices de terceiros.

#### Histórico da adaptação `number.go`

O proxy Go identifica `go-humanize v1.1.0` pelo commit
`40736da3e8ed16369b5e6f109548f243d43a7e34`. O histórico GitHub de
`number.go` nessa tag identifica sua introdução no commit
[`2b41dc516fc83c0d0cb79dbbaed05f61ce6d0ff8`](https://github.com/dustin/go-humanize/commit/2b41dc516fc83c0d0cb79dbbaed05f61ce6d0ff8),
com data de autoria 2015-01-16. O diff de introdução já contém o crédito a
gorhill, o link do gist e a frase de adaptação; há commits posteriores no
histórico do arquivo, portanto o fonte atual não deve ser tratado como idêntico
ao arquivo introduzido.

A API do gist lista como última revisão anterior à introdução de `number.go`
[`cc36e402754549e658cf1d7255fd84f3c284f063`](https://gist.github.com/gorhill/5285193/cc36e402754549e658cf1d7255fd84f3c284f063),
de 2013-09-15, com `render_number.go` e `~LICENSE` contendo WTFPL v2. O
arquivo fonte nessa revisão também afirma `Feel free to reuse as you wish`.
Comparando o campo textual `render_number.go` retornado pela API para essa
revisão e para as três revisões anteriores consultadas, inclusive uma de
2013-04-07, os quatro textos coincidiram; os arquivos de licença associados
nessas revisões diferem: duas exibem `LICENSE`/`~LICENSE` WTFPL e duas não
listam arquivo de licença. Isso documenta avisos contemporâneos possíveis,
não comprova qual revisão foi efetivamente consultada pelo adaptador nem que
o campo de texto da API seja uma cópia autenticada byte a byte do arquivo
original usado em 2015.

O commit de introdução confirma a alegação de adaptação, mas não determina
autoria integral, compatibilidade ou condições aplicáveis às modificações e
artefatos. Preservar o crédito e os textos relevantes para revisão jurídica;
não substituir automaticamente licenças no SBOM ou declarar resolvidos os
direitos. O arquivo foi selecionado em `go list`, mas sua presença efetiva no
executável histórico não foi comprovada por link map.

#### Comparação textual da introdução com o gist

Foram obtidos `number.go` no commit de introdução
`2b41dc516fc83c0d0cb79dbbaed05f61ce6d0ff8` e `render_number.go` na revisão
do gist `cc36e402754549e658cf1d7255fd84f3c284f063`; ambas as respostas HTTP
foram 200. Após remover espaços das extremidades e linhas vazias, 140 das
168 linhas do arquivo introduzido aparecem entre as 163 linhas do gist. Essa
contagem permite repetições e ignora ordem, portanto é uma medida textual
limitada, não percentual de código copiado ou equivalência.

Ambos contêm as tabelas `renderFloatPrecisionMultipliers` e
`renderFloatPrecisionRounders` e o identificador `formatDirectiveIndices`.
O gist declara `RenderFloat` e `RenderInteger`; o arquivo introduzido declara
`FormatFloat` e `FormatInteger`, adiciona `package humanize` e mantém o crédito
ao autor e link do gist. A frase `Feel free to reuse as you wish` está no
comentário do gist, mas não aparece no `number.go` introduzido. A revisão do
gist também contém `~LICENSE` WTFPL v2 conforme registrado acima; o `LICENSE`
do módulo é MIT. As diferenças não demonstram por si só remoção indevida de
aviso, compatibilidade de licenças ou termos juridicamente aplicáveis.

A correspondência é coerente com a adaptação declarada, mas não estabelece
qual revisão do gist foi usada, nem revisa todas as modificações posteriores.
O texto selecionado pelo build atual e sua retenção no binário histórico são
questões separadas; avaliar integralmente os avisos para cada source bundle e
artefato escolhido antes da distribuição. Nenhum código externo foi executado.

### `modernc.org/mathutil v1.7.1`: fontes selecionados

Com Go 1.27.3, Linux amd64 e resolução offline, `go list -mod=readonly`
selecionou 13 arquivos Go no pacote raiz: `binarylog.go`, `bits.go`,
`envelope.go`, `int.go`, `mathutil.go`, `permute.go`, `poly.go`, `primes.go`,
`rat.go`, `rnd.go`, `sqr.go`, `tables.go` e `test_deps.go`; `sqr_std.go` foi
ignorado. Os imports informados foram `fmt`, `math`, `math/big`, `sort` e
`github.com/remyoudompheng/bigfft`. Apesar do nome, `test_deps.go` aparece
na seleção normal de `GoFiles`; isso não prova que cada símbolo seja vinculado.

Doze dos 13 arquivos selecionados apresentam nos cabeçalhos copyright The
mathutil Authors e remetem ao `LICENSE`; `sqr.go` não apresenta esse cabeçalho
nas primeiras linhas e contém uma chamada a `bigfft.Mul`. Essa ausência não
estabelece licença diferente ou ausência de direitos. O `LICENSE` principal
contém BSD de três condições e copyright 2014 The mathutil Authors, incluindo
condições para distribuição de fontes e binários e disclaimer. Os avisos de
`bigfft` foram triados separadamente acima.

Essa seleção atual não é prova das flags históricas nem link map do executável
de conformance previamente inventariado. Outros targets, tags, testes,
subpacotes e source bundles podem conter outros arquivos ou avisos; esta
inspeção não é auditoria jurídica completa. Revisar textos e direitos para
cada artefato efetivamente distribuído. Nenhum SBOM ou contrato foi alterado.

### `golang.org/x/sys/unix v0.49.0`: arquivos selecionados

No checkout atual, `go list -mod=readonly` com Go 1.27.3, `GOOS=linux`,
`GOARCH=amd64` e `GOPROXY=off` selecionou **42 arquivos Go e um assembly**
para `golang.org/x/sys/unix`; não selecionou arquivos CGo. Uma busca nos
primeiros 16 versos de cada arquivo encontrou `The Go Authors` em 35 dos 43.
Os oito restantes são `zerrors_linux.go`, `zerrors_linux_amd64.go`,
`zptrace_x86_linux.go`, `zsyscall_linux.go`, `zsyscall_linux_amd64.go`,
`zsysnum_linux_amd64.go`, `ztypes_linux.go` e `ztypes_linux_amd64.go`.
Dois cabeçalhos amostrados os identificam como gerados; nenhum dos oito exibe
a expressão procurada nos primeiros 16 versos. Ausência nesse trecho não prova
ausência de direitos, licença ou avisos de terceiros nos fontes ou insumos.

O `LICENSE` do módulo contém BSD de três condições; o arquivo `PATENTS` traz
uma concessão adicional de direitos de patente pelo Google, com escopo e
condição de término definidos em seu próprio texto. Revisar ambos na íntegra,
assim como os geradores e eventuais headers de sistema usados pelos arquivos
gerados, para os fontes e binários escolhidos; não presumir que um resumo de
licença cubra todos os insumos ou que o texto de patentes se aplique fora de
seus próprios termos. Esta triagem não revisou integralmente os 43 arquivos.

O binário Go de conformance histórico já inventariado lista `x/sys v0.49.0`;
esta seleção atual é evidência de configuração, não captura das flags do build
histórico, link map ou prova de que cada função permaneceu no binário. Outros
targets, build tags e pacotes de fontes podem alterar o conjunto. Não se
modificaram SBOMs ou montou pacote de notices; a revisão por artefato e a
confirmação jurídica continuam pendentes.

#### Geradores dos oito arquivos `x/sys/unix`

Os oito cabeçalhos foram lidos no módulo local; a tabela registra os comandos
ou geradores **declarados** neles, não execução observada ou reprodução:

| Arquivo | Gerador ou insumo indicado |
|---|---|
| `zerrors_linux.go`, `zsyscall_linux.go`, `ztypes_linux.go` | `mkmerge`, que `linux/mkall.go` invoca sobre as variantes por arquitetura. |
| `zerrors_linux_amd64.go` | `mkerrors.sh` com includes em `/tmp/amd64/include`. |
| `zptrace_x86_linux.go` | `linux/mkall.go generatePtracePair("386", "amd64")`; o gerador extrai definições das variantes de arquitetura. |
| `zsyscall_linux_amd64.go` | `mksyscall.go` a partir de `syscall_linux.go`, `syscall_linux_amd64.go` e `syscall_linux_alarm.go`. |
| `zsysnum_linux_amd64.go` | `linux/mksysnum.go` com `asm/unistd.h` no diretório temporário de includes. |
| `ztypes_linux_amd64.go` | `cgo -godefs` sobre `linux/types.go`, seguido de `mkpost.go`. |

Os cabeçalhos locais de `linux/mkall.go`, `linux/mksysnum.go`,
`mksyscall.go`, `mkpost.go`, `mkerrors.sh`, `internal/mkmerge/mkmerge.go` e
`linux/types.go` creditam The Go Authors e remetem ao `LICENSE` do módulo.
O `unix/README.md` descreve geração Linux a partir de fontes de kernel e
bibliotecas; o `linux/Dockerfile` **presente nesta versão** aponta Linux v7.2,
branch glibc `release/2.44/master` e Go 1.26.8. Uma branch mutável e caminhos
temporários não fixam todos os commits ou headers efetivamente usados para
gerar esses arquivos. A presença desse Dockerfile não demonstra que o build
histórico o executou. Nenhum gerador, container ou código externo foi executado
nesta investigação.

Os textos BSD e `PATENTS` identificados acima, os headers e os avisos dos
insumos de geração exigem revisão conforme as fontes ou binários realmente
distribuídos. Não se conclui que licenças de ferramentas se transfiram
automaticamente ao output, nem que todos os 43 arquivos selecionados estejam
no binário de conformance. A comparação de geração reproduzível, a proveniência
exata dos headers e a revisão jurídica por artefato permanecem pendentes.

#### Amostras upstream dos headers de geração

`linux/types.go` inclui explicitamente `<linux/tcp.h>`, `<dirent.h>` e
`<unistd.h>`, entre muitos outros headers. O comando registrado em
`zsysnum_linux_amd64.go` menciona `/tmp/amd64/include/asm/unistd.h`. Como
amostras das fontes citadas pelo Dockerfile atual de `x/sys`, foram lidos:

| Fonte consultada | Aviso observado |
|---|---|
| [Linux v7.2 `arch/x86/include/uapi/asm/unistd.h`](https://github.com/torvalds/linux/blob/v7.2/arch/x86/include/uapi/asm/unistd.h) | `GPL-2.0 WITH Linux-syscall-note`; o header seleciona outros headers de arquitetura conforme macros. |
| [Linux v7.2 `include/uapi/linux/tcp.h`](https://github.com/torvalds/linux/blob/v7.2/include/uapi/linux/tcp.h) | `GPL-2.0+ WITH Linux-syscall-note`, com crédito a Fred N. van Kempen e aviso GPL no próprio arquivo. |
| [glibc `release/2.44/master` `dirent/dirent.h`](https://sourceware.org/git/?p=glibc.git%3Ba=blob%3Bf=dirent/dirent.h%3Bhb=refs/heads/release/2.44/master) e [ `posix/unistd.h`](https://sourceware.org/git/?p=glibc.git%3Ba=blob%3Bf=posix/unistd.h%3Bhb=refs/heads/release/2.44/master) | Cabeçalhos copyright Free Software Foundation e LGPL 2.1 ou posterior. A branch é mutável. |

O texto upstream [`Linux-syscall-note`](https://github.com/torvalds/linux/blob/v7.2/LICENSES/exceptions/Linux-syscall-note)
descreve a exceção para headers UAPI e o uso normal de chamadas de sistema.
Consultar os textos completos e sua aplicabilidade, sem generalizar a exceção
para qualquer trecho de kernel ou concluir que os termos dos headers se
transfiram automaticamente ao código gerado. Esses quatro arquivos são
**amostras de fontes possíveis**, não uma identificação comprovada dos headers
reais gerados/instalados em `/tmp/amd64/include` no build histórico: o
Dockerfile não fixa commit imutável da branch glibc nem todas as dependências.
Os includes transitivos e demais headers não foram auditados. Não foram
executados geradores ou compiladores nesta etapa; a seleção de código e os
notices de distribuição exigem verificação por artefato e revisão jurídica.

#### Amostra de includes transitivos

Na árvore Linux v7.2 consultada, `include/uapi/linux/tcp.h` inclui
`<linux/types.h>`, `<asm/byteorder.h>` e `<linux/socket.h>`. Os arquivos
upstream [`linux/types.h`](https://github.com/torvalds/linux/blob/v7.2/include/uapi/linux/types.h),
[`linux/socket.h`](https://github.com/torvalds/linux/blob/v7.2/include/uapi/linux/socket.h)
e [`asm/byteorder.h` x86](https://github.com/torvalds/linux/blob/v7.2/arch/x86/include/uapi/asm/byteorder.h)
declaram `GPL-2.0 WITH Linux-syscall-note`. O último inclui
[`linux/byteorder/little_endian.h`](https://github.com/torvalds/linux/blob/v7.2/include/uapi/linux/byteorder/little_endian.h),
com o mesmo SPDX; `linux/types.h` inclui `asm/types.h` e
[`linux/posix_types.h`](https://github.com/torvalds/linux/blob/v7.2/include/uapi/linux/posix_types.h),
este também com o mesmo SPDX. Uma tentativa direta de localizar
`arch/x86/include/uapi/asm/types.h` no snapshot retornou 404: não se identificou
por essa tentativa o caminho instalado efetivo para `asm/types.h`.

Nos exemplos glibc já examinados, `dirent/dirent.h` e `posix/unistd.h` incluem
`<features.h>`; a amostra upstream
[`include/features.h`](https://sourceware.org/git/?p=glibc.git%3Ba=blob%3Bf=include/features.h%3Bhb=refs/heads/release/2.44/master)
na branch `release/2.44/master` declara copyright Free Software Foundation e
LGPL 2.1 ou posterior. A branch é mutável; não foi identificado o commit ou
header instalado do build histórico. As referências demonstram caminhos
textuais **possíveis**, sujeitos a processamento de headers, condições e
sysroot, não a cadeia efetivamente consumida por um compilador. Esta amostra
não percorre toda a árvore transitiva, não verifica licenças de todos os
insumos e não prova vínculo no executável ou obrigações de distribuição.
Nenhum gerador foi executado; preservar a revisão jurídica por artefato.

#### Origem prevista de `asm/types.h` no Linux x86

Na árvore Linux v7.2, [`include/uapi/asm-generic/Kbuild`](https://github.com/torvalds/linux/blob/v7.2/include/uapi/asm-generic/Kbuild)
lista `types.h` como header UAPI obrigatório. O
[`scripts/Makefile.asm-headers`](https://github.com/torvalds/linux/blob/v7.2/scripts/Makefile.asm-headers)
prevê fallback para `asm-generic` quando a arquitetura não possui o header e
gera um wrapper `#include <asm-generic/types.h>`. O
[`Kbuild` UAPI x86](https://github.com/torvalds/linux/blob/v7.2/arch/x86/include/uapi/asm/Kbuild)
lista headers `unistd` gerados, não `types.h`; o caminho de fonte x86 direto
consultado anteriormente retornou 404. Essas regras sustentam o wrapper
gerado como origem prevista para x86 no snapshot examinado.

O [`Makefile` principal](https://github.com/torvalds/linux/blob/v7.2/Makefile)
faz `headers` depender de `uapi-asm-generic`; o
[`scripts/Makefile.headersinst`](https://github.com/torvalds/linux/blob/v7.2/scripts/Makefile.headersinst)
descreve exportação de headers UAPI de origem e gerados para `usr/include`,
com processamento pelo instalador. A fonte
[`include/uapi/asm-generic/types.h`](https://github.com/torvalds/linux/blob/v7.2/include/uapi/asm-generic/types.h)
declara `GPL-2.0 WITH Linux-syscall-note` e inclui
[`asm-generic/int-ll64.h`](https://github.com/torvalds/linux/blob/v7.2/include/uapi/asm-generic/int-ll64.h),
que declara a mesma expressão e inclui `asm/bitsperlong.h`. Os Makefiles
citados declaram SPDX `GPL-2.0`; suas licenças e a dos headers devem ser
avaliadas nos respectivos contextos, sem atribuição automática ao output.

Isto é análise das regras e fontes upstream, não execução de `headers_install`
ou identificação dos bytes que estavam em `/tmp/amd64/include` quando `x/sys`
foi gerado. Outros headers, configuração e transformação do instalador não
foram auditados integralmente. Não se provou vínculo em artefato final ou
obrigação específica de distribuição; manter revisão jurídica por artefato.

Os arquivos `LICENSE` principais examinados no cache desses nove módulos
indicam MIT para `go-humanize` e textos BSD de três condições para os outros
oito. A evidência dos SBOMs `go.cdx.json` e `go-with-tests.cdx.json` indica
**Apache-2.0** para `modernc.org/sqlite` e **MIT** para `modernc.org/libc`,
apesar de seus respectivos arquivos `LICENSE` principais serem BSD de três
condições. Não foi estabelecida a causa da divergência nem se a evidência
automática se refere a um componente interno. Não substituir esses valores
automaticamente: investigar o gerador, preservar proveniência e produzir novo
SBOM/revisão de notices do artefato de release.

O arquivo `modernc.org/sqlite@v1.61.1/LICENSE-3RD-PARTY.md` discrimina SQLite
3.54.1 traduzido para Go, licenças dos componentes e textos herdados; também
declara `sqlite-vec` e o VFS como condicionais aos respectivos imports. Sua
matriz cita `modernc.org/libc` 1.79.0, enquanto o binário usa **1.80.1**: não
assumir que o notice transitivo de uma versão cobre automaticamente a outra.
O arquivo `LICENSE-SQLITE` apresenta a declaração upstream de dedicação ao
domínio público; a aplicabilidade e o pacote de atribuição dependem de revisão.
`modernc.org/libc@v1.80.1/LICENSE-3RD-PARTY.md` inclui textos herdados como
Go e musl; `modernc.org/memory@v1.12.1` inclui `LICENSE-GO` e
`LICENSE-MMAP-GO`. Determinar quais partes atingem o artefato e preservar os
avisos aplicáveis é trabalho pendente. Os textos BSD examinados exigem, para
redistribuição binária, reproduzir aviso de copyright, condições e disclaimer
na documentação e/ou materiais distribuídos; confirmar todos os textos originais.

### Matriz SQLite e notice libc fixado

Foi comparado o apêndice `N1` de
`modernc.org/sqlite@v1.61.1/LICENSE-3RD-PARTY.md` com
`modernc.org/libc@v1.80.1/LICENSE-3RD-PARTY.md`, ambos no cache local cujos
SHA-256 estão abaixo. Após remover três `#` dos títulos Markdown de N1 e
remover o separador do apêndice, os conteúdos tiveram **305 linhas idênticas**.
Sem essa normalização, os títulos e o separador diferem; não se afirma
identidade byte a byte dos arquivos completos. Ambos apresentam notices para
Go, musl libc, go-netdb e NixOS/nixpkgs; N1 aponta a esses mesmos quatro
componentes. A matriz SQLite identifica `libc v1.79.0`, enquanto o binário
Go de conformance inventariado usa `v1.80.1`: a coincidência do texto N1 com
o notice desta última é uma evidência concreta sobre **esse arquivo**, não
prova de identidade das árvores libc ou cobertura de todos os avisos.

A matriz upstream classifica diversos componentes como vinculados, mas
qualifica `sqlite-vec` e `vfs` como dependentes de import explícito; também
lista material de testes, módulo apenas no grafo e arquivos fora do build.
Essas categorias são declarações do upstream, não inspeção do link map de
cada produto. Para o binário de conformance, a lista prévia de módulos e a
comparação textual não identificam todas as funções/arquivos incorporados;
outros binários, testes ou um source bundle podem incluir conteúdo diferente.
Revisar notices completos, escolhas aplicáveis, configurações e direitos de
cada artefato com o responsável jurídico. Os SBOMs originais e os de
quarentena permanecem inalterados; não foi montado pacote final de notices.

### `modernc.org/memory v1.12.1`: avisos e seleção de fontes

No cache local, `LICENSE` contém texto BSD de três condições com copyright
The Memory Authors; `LICENSE-GO` contém texto BSD de três condições com
copyright The Go Authors; `LICENSE-MMAP-GO` contém texto BSD de três condições
com copyright Evan Shaw. Seus textos exigem preservar os avisos, condições e
disclaimers nas formas descritas em cada licença. `mmap_unix.go` credita Evan
Shaw, menciona modificações de The Memory Authors e referencia expressamente
`LICENSE-MMAP-GO`. Uma busca textual no módulo não identificou uma referência
de arquivo de código específica a `LICENSE-GO`; sua presença não deve ser
confundida com prova de que cada fonte do módulo tenha origem no Go.

Com Go 1.27.3, `GOOS=linux`, `GOARCH=amd64`, `GOPROXY=off` e
`go list -mod=readonly` no módulo Go atual, a seleção de `modernc.org/memory`
foi `decommit_unix.go`, `memory.go`, `memory64.go`, `mmap_unix.go`,
`nocounters.go` e `trace_disabled.go`; os arquivos Windows, `memory32.go` e
variantes de trace/counters foram ignorados nessa consulta. Essa é a seleção
atual para essa configuração, não um registro das flags do build histórico nem
prova de retenção de cada função no executável. O binário de conformance Go já
inventariado lista o módulo v1.12.1; outros binários, targets, tags e testes
podem selecionar código diferente.

`LICENSE-LOGO` contém apenas um link para uma página Wikimedia e o módulo inclui
`logo.png`. A presença desse arquivo não estabelece por si só a licença da
imagem, sua inclusão no binário ou a possibilidade de distribuí-la; verificar
origem, termos, versão e eventuais atribuições se um source bundle a incluir.
Os avisos `LICENSE-GO` e `LICENSE-MMAP-GO` devem ser revisados com os demais
textos e arquivos efetivamente distribuídos; a lista não constitui parecer
jurídico ou notice final. Nenhum SBOM foi modificado.

#### Investigação da imagem indicada por `LICENSE-LOGO`

A [página Wikimedia Commons indicada pelo módulo](https://commons.wikimedia.org/w/index.php?title=File:Memory_infra_logo.png&oldid=829804081)
atribui a imagem original ao Chromium Project e informa uma versão retocada por
Rezonansowy para remover ícones Chrome. A página declara **CC BY 2.5 para o
original** e **CC0 1.0 para as alterações de remoção dos ícones**; não tratar
o CC0 das alterações como renúncia automática aos direitos do original. A
página descreve atribuição, link à licença e indicação de alterações como
condições para o original. Essas são declarações da página, não verificação
independente de titularidade ou parecer jurídico.

A API Commons informa para a imagem corrente dimensões 323×150, 11.438 bytes
e SHA-1 `493769865c77176f9eb891353d11d45c05ef2612`. O `logo.png` local de
`modernc.org/memory@v1.12.1` tem dimensões **200×93** e `sha1sum`
`a630d19f41ddb0210c9a7f7c4c254032700ce208`: não é a mesma sequência de bytes.
A inspeção visual e o link de `LICENSE-LOGO` sugerem relação, mas não
estabelecem qual transformação ou fonte exata produziu o arquivo local, nem
seus direitos. SHA-1 aqui é usado para comparação com metadados Commons, não
como garantia criptográfica de origem.

A análise é relevante se uma distribuição incluir a imagem ou fontes do
módulo: confirmar procedência da versão local, direitos, modificações e
atribuição/condições aplicáveis antes de distribuí-la. A imagem não foi
identificada como incorporada ao executável Go de conformance; sua presença no
cache ou o módulo constar no binário não demonstram que esteja nele. Não se
alterou nem redistribuiu a imagem e não se concluiu uma licença final.

**Histórico upstream e versões Commons:** o proxy Go identifica a tag
`modernc.org/memory v1.12.1` com o commit GitLab
`bb3d99379ed7361ae4c08f11c2876aef046c2a18`. A API GitLab informa para
`logo.png` nessa revisão SHA-256
`17bb2fd3442aeb1f4a17554ac930ff9d6060cca69dde043dd621327ac76e0880`,
igual ao `sha256sum` do arquivo no cache local. A API informa o mesmo hash no
commit de introdução [`7dfb518a0259223c5c5cbc2adb6bc6614d061e6b`](https://gitlab.com/cznic/memory/-/commit/7dfb518a0259223c5c5cbc2adb6bc6614d061e6b)
de 2025-03-20, cujo diff adiciona tanto o binário `logo.png` quanto
`LICENSE-LOGO` com o link Commons. A consulta de histórico por esse caminho
na revisão fixada retornou apenas esse commit. A correspondência com o hash
informado pela API identifica a cópia upstream do módulo, mas o diff binário
e a mensagem `add logo` não descrevem transformação nem provam direitos.

A API de histórico de arquivos do Commons retornou **uma revisão**: upload de
2015-12-19 14:34:01 UTC por Rezonansowy, com 323×150 e SHA-1 já registrado
acima. Não foi encontrada ali uma revisão 200×93 correspondente ao arquivo
local. Isso não exclui thumbnail, transformação fora do Commons ou outra fonte;
nenhuma dessas hipóteses foi comprovada. O histórico reforça a referência
upstream à página e a identidade do arquivo no módulo, mas a origem exata da
versão reduzida, condições aplicáveis e direitos continuam a verificar se um
artefato incluir a imagem. Nenhum arquivo foi publicado ou substituído.

**Tentativa de comparação com thumbnail de 200 pixels:** a API Commons,
consultada com `iiurlwidth=200`, informou dimensões de apresentação 200×93,
mas retornou URL de thumbnail de **250 pixels**. A solicitação direta ao URL
`200px-Memory_infra_logo.png` retornou HTTP 400 com orientação para usar os
tamanhos suportados; portanto não foi obtido um thumbnail Commons nativo de
200 pixels e não se afirma comparação direta com ele. O thumbnail de 250 pixels
obtido tem dimensões 250×116, 16.539 bytes e SHA-256
`ff401189fc8ddb2bf04b84f083b5c7130f6543cfc1b1d3c4cce6fd204b582506`,
diferente do arquivo local de 11.369 bytes e SHA-256 já registrado.

Como sondagem limitada, o thumbnail de 250 pixels foi redimensionado em memória
para 200×93 usando Pillow e Lanczos e comparado como RGBA com o PNG local:
**8.756 de 18.600 pixels diferiram**. Esse resultado não exclui a mesma imagem
original, pois redimensionamento, transparência e versões dos processadores
podem alterar pixels; não identifica a transformação histórica. O ensaio não
gravou nem distribuiu a imagem, não determina sua licença e não altera a
necessidade de revisão de proveniência caso seja incluída em artefato.

## Rust e bibliotecas do sistema

`nm -C` encontrou símbolos `sqlite3_open`, `sqlite3_open16` e `sqlite3_open_v2`
no executável Rust. O crate `libsqlite3-sys` 0.39.1 tem licença MIT para o
wrapper; sua amalgamação `sqlite3/sqlite3.c` identifica SQLite 3.54.1 e contém
disclaimers dos autores. Não confundir a licença MIT do wrapper com a declaração
de domínio público do SQLite. Revisar o conteúdo incorporado e suas condições.

O lockfile e o SBOM Rust abrangem `aws-lc-sys` 0.47.0. Seu `LICENSE` descreve
origens AWS-LC, BoringSSL e OpenSSL e diversos componentes, incluindo fiat-crypto,
com textos e atribuições próprios. Uma busca textual de símbolos `aws_lc`,
`aws_lc_rs` e `aws_sdk_s3` no binário de conformance não encontrou resultados;
isso **não prova ausência** após otimização/remoção de símbolos e não cobre
aplicações que realmente usam S3. Inspecionar separadamente cada artefato de
distribuição com suas features e incluir notices de código efetivamente entregue.

`readelf -d` identificou dependências dinâmicas `libresolv.so.2` e `libc.so.6`
no executável Go; no Rust, `libgcc_s.so.1`, `libm.so.6`, `libc.so.6` e o
loader `ld-linux-x86-64.so.2`. São nomes referenciados, não prova de que essas
bibliotecas sejam empacotadas. Se uma imagem ou distribuição as incluir, revisar
suas versões, código-fonte correspondente e obrigações aplicáveis; não inferir
que apenas a inspeção dos binários satisfaça tais obrigações.

## Fontes verificadas

Arquivos de origem em caches locais, referidos por módulo e caminho relativo;
os hashes registram os bytes examinados, não garantem procedência legal:

| Componente / arquivo | SHA-256 |
|---|---|
| `modernc.org/sqlite@v1.61.1/LICENSE` | `c6fe05491a60ae13bcd223088d2705e36dede24e5587226231d2459ada5c4822` |
| `modernc.org/sqlite@v1.61.1/LICENSE-3RD-PARTY.md` | `4194f8e7119be72babd499db4f6f1f7547e36e8a7bffcb2b9c8df241b8f1576a` |
| `modernc.org/sqlite@v1.61.1/LICENSE-SQLITE` | `8438c9c89b849131ead81d5435cb97fcf052df5b0b286dda8a2d4c29e6cb3fd0` |
| `modernc.org/libc@v1.80.1/LICENSE` | `95ff867eb55a56935fa7492406cfa953fb7c13ca73f4c0a86ae05756b4605600` |
| `modernc.org/libc@v1.80.1/LICENSE-3RD-PARTY.md` | `f597097efe3d97021f89170746bd3a0fb9a8b6fb26b82043ed68a4e0283bee6c` |
| `libsqlite3-sys-0.39.1/LICENSE` | `c10c1f27337546471e5f7e4e97fdd398b35b9d4e126115dcd22de8d8e65abf6f` |
| `aws-lc-sys-0.47.0/LICENSE` | `728536b4160e051f86d7c9c388f704866b3d512cd7df97ac3516c65279523c4e` |

Faltam inventário e avisos para todos os produtos/targets/features realmente
entregues, revisão completa dos arquivos e fontes nativas, investigação das
divergências no SBOM, direitos das fixtures e aprovação do mantenedor. Não criar
`LICENSE`, pacote legal final, release ou tag com base nesta revisão parcial.
