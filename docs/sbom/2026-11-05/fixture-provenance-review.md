# Proveniência das fixtures — revisão técnica parcial

Levantamento em 2026-11-05 dos arquivos rastreados em `conformance/fixtures/`;
não é parecer jurídico, comprovação de autoria nem autorização de distribuição.
O conjunto efetivamente distribuível ainda precisa ser definido pelo mantenedor.
Não há licença final do projeto aprovada para publicação nesta revisão.

| Conjunto | Quantidade de dados | Evidência de geração e conteúdo |
|---|---:|---|
| `ltx-v2/snapshot-*.ltx` | 5 | `conformance/generate-ltx-v2-fixture.sh` cria banco com `go/cmd/conformance` e o codifica pelo oráculo `conformance/reference`; 128 linhas com IDs e payloads sintéticos. |
| `ltx-v2/recovery-chain-4096/` | 4 | `generate-recovery-chain-fixtures.py` cria banco e alterações sintéticas com Python/SQLite e codifica snapshot/deltas via oráculo. |
| `ltx-v2/large-transaction-4096/` | 2 | Mesmo gerador: payloads de 8.192 bytes derivados de SHA-256 sobre strings sintéticas e IDs. |
| `ltx-v2/long-chain-4096/` | 33 | `generate-long-chain-fixtures.py` cria snapshot e 32 deltas de valores `step-NNN`, via oráculo; `SHA256SUMS` acompanha os dados. |
| `wal/*.wal` | 3 | `generate-wal-fixtures.py` constrói bytes e checksums do formato a partir de padrões numéricos locais; não é captura de banco de cliente. |

Os geradores LTX referenciam `conformance/reference/go.mod`, que fixa
`github.com/superfly/ltx v0.4.0` e `github.com/pierrec/lz4/v4 v4.1.22`;
`conformance/reference/main.go` chama esse encoder. A cópia local do pacote
`superfly/ltx` inclui `LICENSE` Apache-2.0 e não apresenta `NOTICE` no diretório
raiz inspecionado; isso não prova inexistência de outros avisos relevantes nem
determina automaticamente a licença de arquivos gerados. Os geradores usam
SQLite e outros componentes, cujos termos e notices precisam ser avaliados no
contexto do pacote de distribuição, conforme `native-notices-review.md`.

Os commits locais identificados por `git log -- conformance/fixtures` incluem
`5aae79a`, `25e2bd1`, `1594687`, `9e9e917` e `a51a613`. São evidência de
histórico no checkout, não prova de titularidade, origem anterior ou autorização
de todos os contribuidores. Os scripts descrevem entradas sintéticas; isso não
prova que cada byte histórico foi efetivamente produzido por eles ou que outros
arquivos fora deste diretório não contenham fixtures distribuíveis.

## Verificações realizadas

- Os 14 hashes de snapshots, cadeias curta e grande e WAL publicados nos dois
  READMEs coincidiram com os arquivos. `sha256sum --check SHA256SUMS` confirmou
  os 33 arquivos da cadeia longa.
- `build_wal` foi executada em memória para os três vetores, sem escrever no
  checkout; os bytes gerados corresponderam aos três WAL rastreados.
- O `git status` não indicou alterações locais nos geradores ou nas fixtures
  examinadas. Hashes e correspondência são evidência de identidade e
  reprodutibilidade limitada, não de direitos de uso ou redistribuição.
- Uma regeneração posterior das 44 fixtures LTX é descrita abaixo; isso não
  identifica por si só o ambiente histórico original nem confirma direitos.

### Regeneração LTX em diretório isolado

Em 2026-11-05, os dois geradores Python de cadeias e `conformance/reference/`
foram copiados para `/tmp/opencode/ltx-fixture-review-nyik1y9y/`, com diretório
de fixtures inicialmente vazio. Os cinco snapshots foram gerados pelo script
existente com destinos explícitos nesse mesmo diretório; o script usa os fontes
Go do checkout e cria seu banco temporário fora dele. O ambiente foi reduzido,
com `GOPROXY=off`, `GOTOOLCHAIN=local` e `GOMAXPROCS=2`: Go 1.27.3
linux/amd64, Python 3.14.8 e SQLite 3.54.0 do módulo Python. O módulo Go usa
`modernc.org/sqlite` 1.61.1; o oráculo fixado usa `superfly/ltx` 0.4.0 e
`pierrec/lz4/v4` 4.1.22. Não se afirma que todas as chamadas utilizem a mesma
implementação SQLite.

As cadeias foram produzidas pelos geradores copiados e verificadas pelo
próprio oráculo; os cinco snapshots foram gerados separadamente. Comparando
SHA-256 por caminho relativo, o resultado foi **44 originais, 44 gerados,
44 iguais, zero diferentes, ausentes ou adicionais**. O resultado corrobora
reprodutibilidade nesse ambiente e com os mesmos scripts e oráculo, não uma
implementação independente do encoder nem prova de autoria ou direitos. Não
foram sobrescritas fixtures existentes; arquivos temporários de investigação
foram preservados. Outra versão de SQLite ou das ferramentas pode gerar bytes
diferentes, e esta execução não substitui homologação ou revisão jurídica.

Na verificação posterior, `go run . verify-ltx <snapshot>` foi executado com o
oráculo copiado e Go 1.27.3 offline para cada snapshot regenerado: tamanhos de
página 512, 1024, 4096, 8192 e 65536, todos **5/5 aprovados**. Em cada caso,
o SHA-256 do arquivo isolado também coincidiu com o arquivo atual no checkout.
O comando usa `Decoder.Verify` do mesmo oráculo usado na geração: validação
nesse escopo não equivale a implementação de decodificação independente, nem
prova direitos de distribuição ou substitui o gate completo. Nenhuma fixture
existente foi modificada nessa verificação.

## Pendências antes de distribuir

1. O mantenedor deve confirmar a titularidade e os direitos dos dados, scripts e
   contribuições e identificar quais fixtures, fontes e binários serão entregues.
2. Revisar os avisos e licenças do oráculo, SQLite, LZ4 e outros componentes
   efetivamente incluídos, inclusive notices transitivos e termos do pacote
   final; não inferir que a licença do gerador seja automaticamente a do output.
3. Revisar eventuais fixtures externas ou dados não encontrados pela busca
   limitada descrita abaixo, inclusive arquivos ignorados ou gerados em CI.
4. Concluir revisão jurídica e obter autorização explícita antes de criar
   licença final, release ou tag. A revisão de admissão e o gate seguem abertos.

## Dados e usos fora de `conformance/fixtures`

No mesmo checkout, `git ls-files` filtrado por caminhos com nomes `fixture`,
`testdata`, `corpus` ou `sample`, e extensões `.ltx`, `.sqlite`, `.sqlite3`,
`.db`, `.wal`, `.bin`, `.dat`, `.jsonl` e `.csv`, não encontrou arquivos
adicionais fora de `conformance/fixtures/`. O mesmo filtro sobre arquivos não
rastreados não ignorados também não encontrou dados adicionais. Isso é uma
busca por convenção de nomes, não inspeção exaustiva de todos os arquivos,
arquivos ignorados, históricos, dependências, CI ou artefatos de distribuição.

| Local | Dados e procedência observada | Decisão pendente |
|---|---|---|
| `conformance/snapshot_matrix.py`, `conformance/test_snapshot_matrix.py` | Bancos temporários criados via SQLite com strings, inteiros, padrões binários, rollback e VACUUM definidos no código; testes usam diretórios temporários. | Confirmar direitos do código e dados gerados se forem incluídos num pacote. |
| `conformance/benchmark-snapshot.py`, `docs/litestream-benchmark-2026-11-05.json` | Harness local não rastreado cria payloads SHAKE-256 de entradas sintéticas; o JSON local não rastreado registra resultados, não o banco de entrada. | Confirmar inclusão, procedência e direitos antes de distribuir resultados ou harness; o binário Litestream é ferramenta externa, não fixture incorporada. |
| `go/ltx/fuzz_test.go` e `go/internal/wal/*test.go` | Seeds mínimos construídos em memória, mais leitura e mutação de fixtures compartilhadas já inventariadas. O cache de fuzz pode conter novos dados gerados e não foi revisado. | Avaliar direitos das fixtures originais e do código de teste; não incluir caches sem inventário próprio. |
| `rust/crates/ltx/src/lib.rs`, `rust/crates/ltx/benches/decoder.rs` | `include_bytes!` incorpora fixtures LTX de `conformance/fixtures` em alvos de teste ou benchmark; o teste de CRC também usa um vetor curto padrão. | Confirmar direitos das fixtures originais e verificar a composição de cada artefato compilado antes de distribuí-lo. |
| `go/replicator/*test.go`, `rust/crates/core/tests/`, exemplos de telemetria e testes S3 | Casos inspecionados criam bancos, valores, nomes de objeto e endpoints locais sintéticos em tempo de teste; testes S3 cruzados também leem a fixture LTX compartilhada. Alguns destes arquivos estão não rastreados. | Revisar cada arquivo e contribuição efetivamente incluídos; verificar que dados temporários ou credenciais reais nunca sejam empacotados. |

A busca textual por `include_bytes!`, `include_str!` e `go:embed` em fontes Go
e Rust encontrou referências Rust às fixtures LTX já inventariadas; não encontrou
outro mecanismo desses três. Ela não detecta todos os padrões possíveis de
embutir, baixar ou gerar dados. Os SBOMs e JSONs de benchmark são artefatos de
evidência separados, não considerados datasets de fixture por esse filtro; seus
conteúdos, metadados e direitos devem ser revisados se forem distribuídos.
Nenhuma dessas observações confirma autoria ou concede licença.

## Fonte oficial SQLite e limites para fixtures

A [página oficial de copyright SQLite](https://sqlite.org/copyright.html),
atualizada em 2026-01-12 segundo a própria página, afirma que os autores
dedicaram ao domínio público o código e a documentação entregáveis do SQLite.
Ela distingue scripts de build que podem ter outras licenças e menciona que a
aceitação de domínio público pode variar por jurisdição. Essa é a posição do
projeto SQLite sobre seu material original, não uma confirmação dos direitos
dos contribuidores deste repositório ou de todos os componentes empacotados.

O gerador local `conformance/generate-wal-fixtures.py` declara construir
vetores a partir do formato publicado e monta cabeçalho, checksum e payloads
numéricos com Python; não chama a biblioteca SQLite nesse processo. A
[documentação oficial do formato SQLite](https://sqlite.org/fileformat2.html)
descreve o formato de banco e WAL. A reprodução anteriormente registrada dos
três WAL confirma correspondência ao gerador local, não demonstra a origem
histórica de cada contribuição nem decide os direitos do gerador ou dos bytes
de saída. As fixtures LTX usam outro pipeline e não estão cobertas por essa
observação. Antes de distribuir qualquer fixture ou ferramenta, confirmar
direitos das contribuições e revisar os materiais, bibliotecas e scripts
efetivamente incluídos; não transferir automaticamente a condição declarada
para SQLite aos arquivos gerados ou ao projeto como um todo.
