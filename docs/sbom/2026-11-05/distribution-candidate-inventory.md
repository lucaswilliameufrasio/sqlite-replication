# Inspeção de candidatos de distribuição — parcial

Em 2026-11-05 foram inspecionados manifests e listas de arquivos; posteriormente
foram gerados pacotes locais apenas para investigação, sem publicação, tags,
releases ou licença final. A ADR 0014 prevê bibliotecas
Go e Rust, mas o mantenedor ainda não definiu o conjunto final de artefatos,
targets, features, testes, exemplos ou imagens distribuíveis. Portanto não é
possível afirmar um inventário completo de dependências efetivamente entregues.
Os SBOMs e notices existentes são evidência preliminar, não aprovação legal.

## Crates Rust

### Candidato local do commit `413e08d`

As archives locais `candidate-413e08d-review-a.tar.gz` e `-b.tar.gz`
foram geradas a partir do checkout no commit `413e08d`, sem alterações
rastreadas, e tiveram SHA-256 idêntico
`ebf437a4983641b104494ea546956ea0e33a640bd225091370f9158b3c8f6657`.
A verificação conferiu os 119 arquivos do manifesto e dois controles, sem
membros adicionais, duplicados, não regulares ou caminhos inseguros, antes
de extrair em diretório isolado novo. A archive inclui 42 arquivos sob `rust/`,
50 sob `conformance/` e 27 sob `docs/`, incluindo três com quarentena no nome.
Não há basename exato `LICENSE`, `NOTICE` ou `COPYING`; esta contagem não
verifica avisos incorporados, proveniência nem obrigações de dependências.
Na extração, `cargo +1.99.1 test --offline --locked --workspace --all-features`
passou os testes não ignorados, incluindo 53 do core e 24 do coordenador.
Foram reutilizados o cache offline e o target do checkout: a execução pode
reutilizar artefatos compilados e não demonstra compilação limpa nem instalação
offline em outra máquina. Os testes opt-in de Floci e CP-01 não foram repetidos.
Esta nota foi escrita após a geração e não integra o candidato examinado.
Direitos, notices finais e autorização para distribuição permanecem pendentes.

### Candidato com orçamento conservador

O candidato local de SHA-256
`4e0411875b58ac41177d7d56338e7a7412561fc72b08372613b0e28a390a41a1`
foi gerado duas vezes com o mesmo hash. Extração isolada verificou os 119
arquivos do manifesto e os dois arquivos de controle. `cargo +1.99.1 test
--locked --workspace --all-features` passou nos testes não ignorados,
incluindo 29 unitários do core e 21 do coordenador. Foram reutilizados o
cache offline de dependências e o target local. Os testes opt-in Floci e
CP-01 ficaram ignorados nessa execução. Esta nota é posterior à archive;
direitos, notices finais e autorização de distribuição permanecem pendentes.

### Candidato local `finalcheck` — validação posterior

O candidato `candidate-20261106-finalcheck-a.tar.gz`, SHA-256
`2be937253723afd9937ccd85dcdb1d1d223cf1165753e4a28a237e4823d12c90`,
foi reproduzido em duas gerações idênticas e extraído em diretório isolado.
A verificação conferiu os 118 arquivos do manifesto, os dois arquivos de
controle e a ausência de membros extras. `cargo +1.99.1 test --locked
--workspace --all-features` passou nos testes não ignorados, incluindo 27
unitários do core e 19 testes do coordenador; usou cache offline e target
reutilizado. Testes opt-in de Floci e CP-01 permaneceram ignorados nessa
execução. Esta nota é posterior à archive e não integra seu conteúdo. O
candidato não está aprovado para distribuição ou confirmação de direitos.

### Cópia local do workspace completo — 2026-11-06

Após a escolha do usuário pelo workspace completo, copiou-se `rust/` sem
`target` e `conformance/fixtures/` para
`~/git-worktrees/sqlite-replication/workspace-bundle-validation-20261106/`,
preservando os caminhos relativos usados pelos testes. Na cópia, com Cargo
1.99.1, cache offline e target separado, `cargo test --locked --workspace
--all-features` passou em todos os testes não ignorados, incluindo 11 testes
do coordenador. Foram ignorados os testes explicitamente dependentes de Floci,
ENOSPC, FUSE, QEMU e processos filhos de testes opt-in; esta execução não os
substitui. A cópia contém as alterações locais existentes no momento, não um
commit identificado nem um pacote de release.

Esse ensaio confirma que esse conjunto de fontes e fixtures é testável no host
com as dependências em cache; não prova autossuficiência offline de dependências
para terceiros, conteúdo de distribuição final, conclusão dos notices ou direitos
das fixtures. Não foi criado pacote público nem concedida autorização para
distribuir. As falhas dos `.crate` individuais abaixo permanecem fatos distintos.

No checkout local, `make test` concluiu em 2026-11-06 com Go 1.27.3 e
Rust/Cargo 1.99.1: formatação, vet, Clippy, testes Go e Rust, três fuzz smokes,
conformance cruzada (16 cenários e 96 restores) e Floci com restart passaram.
Parte dos testes Go retornou resultado em cache. Os drills CP-01 ENOSPC, FUSE
e QEMU não fazem parte desse alvo e não foram repetidos nessa execução; outros
testes explicitamente ignorados também não são automaticamente cobertos. O
resultado é do checkout modificado, não prova de homologação de adapters reais,
direitos de distribuição ou compatibilidade universal S3.

Em seguida, `go test -count=1 ./...`, com Go 1.27.3 e `GOPROXY=off`, passou
sem cache de resultados de testes para os pacotes Go do checkout.

### Archive local para revisão — não distribuível

`conformance/build-rust-workspace-candidate.py` cria uma archive local sem
sobrescrever destino, com arquivos rastreados e não ignorados não rastreados
dos diretórios `rust/`, `conformance/fixtures/` e `docs/sbom/2026-11-05/`.
O conteúdo inclui alterações locais, fixtures e SBOMs originais e em
quarentena; `target`, `.git`, arquivos ignorados e outros diretórios do
checkout não são incluídos. Links simbólicos e nomes `.env*` são recusados.
Essa seleção não é uma auditoria de segredos nem uma aprovação de todos os
arquivos. Metadados tar e gzip são normalizados; permissões executáveis são
preservadas como categoria. `SHA256SUMS` registra cada arquivo incluído e
`CANDIDATE-WARNING.txt` identifica que o pacote não está aprovado para
distribuição. A archive não inclui dependências vendorizadas ou licença final.

Duas gerações consecutivas do mesmo checkout em 2026-11-06 produziram 117
arquivos inventariados e SHA-256 idêntico
`6ed8433053734727a438b3027e18809b99a16d6bda3a9dc20cdd71fcbb257f31`.
Extração local isolada verificou os 117 hashes e recusou membros não regulares
ou caminhos inseguros. Na archive extraída, `cargo +1.99.1 test --locked
--workspace --all-features`, com `CARGO_NET_OFFLINE=true` e target próprio,
passou todos os testes não ignorados; o core executou 23 unitários e 11 testes
do coordenador. Testes opt-in dependentes de Floci, ENOSPC, FUSE e QEMU
permaneceram ignorados nessa execução. O cache local de dependências foi usado;
isso não demonstra instalação offline em outra máquina. Alterações
no checkout, inclusive neste documento após a geração, mudam o candidato e
exigem nova geração e validação. A archive é evidência interna de investigação;
notices são preliminares, e direitos das contribuições e fixtures, licença,
composição final e autorização de distribuição seguem pendentes.

Depois das mudanças de drenagem e cancelamento, duas novas archives locais
produzidas em 2026-11-06 tiveram 118 arquivos inventariados e o mesmo SHA-256
`3242927e50213a102b04fda9ac5cb816da76704cb58c2d47af692618ee998b91`.
A extração isolada verificou os 118 hashes e manteve o aviso de candidato não
aprovado. No candidato atualizado extraído, `cargo +1.99.1 test --locked
--workspace --all-features` passou para todos os testes não ignorados, com
24 testes unitários do core e 14 testes do coordenador; Floci e drills CP-01
permaneceram ignorados nessa execução. O teste usou cache offline de
dependências e target separado. Esta atualização
documental ocorreu depois da geração e não está na archive já produzida.

Após acrescentar o teste de timeout e explicitar a ausência de reserva no gate,
duas gerações produziram 118 arquivos e SHA-256 idêntico
`cc353ba69dc4378e840fbee41002f96788352aade6c7db15e1b82e8c13d59c54`.
A extração isolada verificou os 118 hashes e preservou o aviso de não
distribuição. Na archive extraída, o teste Rust completo offline com todas as
features passou nos testes não ignorados, incluindo 15 do coordenador; testes
opt-in Floci e CP-01 ficaram ignorados nessa execução. A execução reutilizou
um target e cache de dependências locais. O gate completo do checkout foi
executado separadamente: `make test` passou com 15 testes do coordenador,
testes Go sem cache, fuzz smoke, conformance e Floci com restart. Drills CP-01
não foram repetidos. Esta nota é posterior à archive.

Após adicionar a referência de capacidade por unidades em voo, duas gerações
locais tiveram 118 arquivos e SHA-256 idêntico
`5539b8a02390f29b8e73dd29e52e6a8090aa1ee63bf8c19a66c24cf452e23bc5`.
A extração verificou os 118 hashes e preservou o aviso de não distribuição.
Na archive extraída, `cargo +1.99.1 test --locked --workspace --all-features`
passou nos testes não ignorados, incluindo 26 unitários do core e 16 testes do
coordenador, com cache e target locais; testes opt-in Floci/CP-01 ficaram
ignorados. `make test` passou separadamente no checkout, incluindo formatação,
vet, Clippy, Go sem cache, fuzz smoke, conformance e Floci com restart; drills
CP-01 não foram repetidos. Esta nota é posterior
à geração da archive.

Após a caracterização adicional de WAL/spool, duas novas archives do checkout
produziram 118 arquivos e SHA-256 idêntico
`598d007d282b84dc86860964b671e05ad776d831cce1cda39817fb53f4198839`.
A extração separada verificou os 118 hashes e o aviso de não distribuição. O
teste Rust offline com todas as features passou nos testes não ignorados,
incluindo 27 unitários do core e 17 testes do coordenador; utilizou cache e
target locais e deixou testes opt-in Floci/CP-01 ignorados. Esta nota é
posterior à archive, que continua sem licença final e
sem aprovação de direitos ou distribuição.

Com Rust/Cargo 1.99.1 e `CARGO_NET_OFFLINE=true`, executou-se
`cargo package --offline --locked --allow-dirty --list -p <crate>` para os sete
crates do workspace principal. `--allow-dirty` foi necessário porque o core
continha alterações locais, incluindo testes e a regressão de admissão; a lista
reflete este checkout, não um commit ou release. O exemplo de telemetria é um
workspace separado com `publish = false`, fora dessa lista. O comando lista
arquivos pretendidos; não prova que um pacote possa ser construído, distribuído
ou publicado, nem enumera código efetivamente vinculado por consumidores.

| Crate | Conteúdo adicional observado na lista |
|---|---|
| `sqlite-replication-ltx` | `src/lib.rs` e `benches/decoder.rs` |
| `sqlite-replication-core` | fontes, CLI de conformance, exemplos e testes, inclusive alterações locais |
| `sqlite-replication-sqlite` | fonte e três arquivos de teste de checkpoint/WAL |
| `sqlite-replication-spool` | fonte |
| `sqlite-replication-storage-fs` | fonte |
| `sqlite-replication-storage-s3` | fonte e teste HTTP de métricas local não commitado |
| `sqlite-replication-wal` | fontes |

Cada lista incluiu `Cargo.toml`, `Cargo.toml.orig`, `Cargo.lock` e
`.cargo_vcs_info.json`; nenhuma listou `LICENSE` ou `NOTICE`. O manifesto
workspace declara Apache-2.0 como intenção, mas isso não substitui os textos e
aprovações pendentes da ADR 0014. O Cargo emitiu aviso de ausência de descrição,
documentação, homepage ou repository nos manifests examinados.

**Risco de pacote incompleto para testes/benchmark:** a lista LTX inclui
`src/lib.rs` e `benches/decoder.rs`, que usam `include_bytes!` para fixtures em
`conformance/fixtures` fora da árvore do crate. Essas fixtures não aparecem na
lista do pacote. Se os arquivos forem extraídos sem o checkout adjacente, esses
alvos não terão as referências disponíveis; isso foi confirmado em um teste do
pacote extraído descrito abaixo, sem implicar falha do build normal da biblioteca.
Revisar de modo semelhante os demais testes que acessam caminhos externos antes
de definir o conteúdo do pacote. Qualquer inclusão das fixtures depende da
confirmação de direitos e dos notices revisados.

As dependências do core incluem os outros crates por `path` nos manifests locais,
além de SQLite, Tokio, S3 e outras bibliotecas. A resolução num pacote publicado
e a estratégia de versões ainda exigem revisão; `--list` não comprova
disponibilidade de dependências no registry. Os SBOMs Rust prévios
cobrem Linux x86-64 e features selecionadas, não todas as dependências de teste,
targets ou binários distribuídos. Native SQLite e AWS-LC requerem a análise de
`native-notices-review.md`; os exemplos de telemetria têm inventário separado.

### Ensaio de empacotamento local e dependências

Posteriormente, com Rust/Cargo 1.99.1, cache offline e target separado, foi
executado `cargo package --offline --locked --allow-dirty --no-verify -p <crate>`
localmente, sem upload. Foram gerados arquivos `.crate` de investigação para
`ltx`, `sqlite`, `storage-fs` e `storage-s3`; `--no-verify` significa que o
Cargo não verificou seu build. `core`, `wal` e `spool` falharam antes da criação
do pacote porque `sqlite-replication-ltx` é dependência `path` sem requisito de
versão. O Cargo informou que uma dependência empacotada usa a versão do registry
e remove `path`; corrigir manifests exigirá uma estratégia de nomes, versões e
publicação revista, não simplesmente assumir que o checkout estará presente.
Essa verificação offline não estabelece disponibilidade real no registry.

O `.crate` LTX foi extraído em diretório isolado sem o checkout adjacente.
`cargo check --offline --locked --lib` **passou**. Em contraste, `cargo test
--offline --locked --lib --no-run` falhou na compilação devido às três referências
externas a `snapshot-512.ltx`, `snapshot-4096.ltx` e `delta-2.ltx` ausentes;
`cargo bench --offline --locked --bench decoder --no-run` falhou por ausência
de `snapshot-4096.ltx` e `large-delta-2.ltx`. Assim, o problema de testes e
benchmark extraídos foi reproduzido, mas não demonstra falha de compilação da
biblioteca normal. Os pacotes locais de investigação não foram publicados nem
constituem artefatos finais; os três restantes foram examinados depois, abaixo.

### Outros três pacotes extraídos

Na investigação seguinte, os `.crate` locais gerados anteriormente para
`sqlite`, `storage-fs` e `storage-s3` foram extraídos sob `/tmp/opencode`, sem
checkout adjacente, e testados com Cargo 1.99.1, `--offline --locked` e target
separado. Os comandos rodaram com ambiente reduzido no host; não houve publicação
nem execução dos testes S3 contra serviço externo.

| Pacote | `cargo check --lib` | `cargo test --no-run` / testes |
|---|---|---|
| `sqlite-replication-sqlite` | Passou | Falhou compilando `checkpoint_locking`: import `sqlite_replication_wal` não resolvido. `wal_capture.rs` também referencia os crates locais LTX e WAL; não foi demonstrado que esse alvo compila. |
| `sqlite-replication-storage-fs` | Não executado separadamente | Compilação e execução passaram: 4 testes unitários; doc-tests sem testes. |
| `sqlite-replication-storage-s3` | Passou | Falhou compilando testes do `src/lib.rs`: `sqlite_replication_ltx` e `sqlite_replication_sqlite` não resolvidos. |

Os manifests originais de `sqlite` e `storage-s3` declaram essas dependências
de desenvolvimento apenas por `path`, sem versão; elas não aparecem nos
respectivos `Cargo.toml` normalizados dos pacotes. Assim, o Cargo conseguiu
gerar os arquivos com `--no-verify`, mas seus testes extraídos não são
autossuficientes. O caminho de fixture S3 cruzada é fornecido por variável de
ambiente no teste opt-in; não foi avaliado um fluxo extraído com tal fixture.
Esses ensaios não comprovam executabilidade dos demais alvos, compatibilidade
com registry nem licenças/notices suficientes para distribuição.

Em uma execução posterior no Linux x86-64, com os pacotes extraídos montados
somente para leitura em Docker rootless sem rede externa, Cargo 1.99.1 e cache
offline, os seguintes alvos independentes passaram:

- `cargo test --offline --locked --lib` no pacote `sqlite`: **8 testes**;
- `cargo test --offline --locked --test checkpoint_crash` no pacote `sqlite`:
  **3 testes**, incluindo recuperação de checkpoints interrompidos;
- `cargo test --offline --locked --test metrics_http` no pacote `storage-s3`:
  **1 teste**, cliente S3 contra servidor HTTP loopback controlado.

Esses resultados isolam os alvos que funcionam das falhas de compilação de
testes por dependências locais omitidas descritas acima. Eles não corrigem nem
exercitam `checkpoint_locking`, `wal_capture` ou os unitários S3 que não
compilam no pacote normalizado. Nenhuma falha adicional de empacotamento foi
observada nesses três alvos; não se conclui que todos os demais alvos funcionem.

### Core, WAL e spool: referências restantes

Uma leitura estática posterior dos manifests e testes identificou o seguinte;
esses três crates não foram extraídos porque `cargo package` já havia recusado
suas dependências normais `path` sem versão:

| Crate/alvo | Referência ou requisito | Efeito observado ou risco |
|---|---|---|
| `core` | Seis dependências normais por `path`: LTX, SQLite, spool, filesystem, S3 e WAL, todas sem versão. | `cargo package --no-verify` falhou no primeiro requisito ausente, LTX. Não foi demonstrada resolução no registry. |
| `spool` | Dependências normais locais LTX, filesystem e WAL, sem versão. | O mesmo erro de empacotamento para LTX; os próximos erros não foram inferidos como saída do Cargo. |
| `wal` | Dependência normal local LTX sem versão; manifesto também declara `publish = false`. | `cargo package --no-verify` falhou pela versão ausente. A política atual não permite publicar WAL em registry; isso afeta a estratégia de distribuição de `core` e `spool`, que dependem dele. |
| `wal/src/lib.rs`, testes | `CARGO_MANIFEST_DIR/../../../conformance/fixtures/wal`, lidos via `fs::read`. | Dependência de arquivos **em execução dos testes**, não `include_bytes!` na compilação. Um pacote isolado sem fixtures no caminho correspondente não atende a esse teste; não foi executado teste extraído. |
| `core/src/recovery.rs`, teste `pinned_reference_recovery_chain_fixtures` | Caminhos relativos para as três árvores `recovery-chain-4096`, `large-transaction-4096` e `long-chain-4096`. | Fixtures são lidas durante o teste; sem o layout adjacente, o teste não terá os dados. Não foi executado teste do pacote extraído. |
| `core` e `spool`, testes opt-in | Variáveis de ambiente e processos filhos para Floci, ENOSPC, FUSE e QEMU. | Uma distribuição isolada não pode ser considerada validada somente por compilar: os harnesses e recursos relevantes precisam ser disponibilizados e autorizados separadamente. |

O mapeamento foi conferido com leitura dos manifests via Python `tomllib` e
busca dos caminhos em fontes Rust. Os manifests de `sqlite` e `storage-s3`
também declaram dependências **de desenvolvimento** somente por `path`; como
demonstrado acima, o Cargo as omitiu dos pacotes normalizados. Não extrapolar o
resultado de `cargo check --lib` para `cargo test`, benchmark ou drills.

Opções para decisão posterior, **não aprovadas**: (1) distribuir fontes como
workspace completo com layout e fixtures autorizadas, sem prometer crates
independentes; (2) preparar crates independentes, definindo estratégia de
versões/registry e resolvendo explicitamente a política `publish = false` do
WAL; ou (3) separar os testes/harnesses dependentes de fixtures do pacote
publicado e mantê-los em um workspace de integração. Qualquer opção deve
preservar cobertura e verificar pacotes extraídos, direitos de fixtures e
notices. Não foram alterados manifests, `publish`, fixtures ou contrato de
admissão nesta investigação; nenhuma opção autoriza publicação.

## Go e demais artefatos

O módulo Go em `go/go.mod` declara dependências, mas consumidores diferentes
selecionam conjuntos de pacotes diferentes. O executável local de conformance
inspecionado em `native-notices-review.md` não representa automaticamente um
binário de produção ou pacote de biblioteca. Não foi identificado pelo filtro de
nomes usado (`Dockerfile`, `.goreleaser`, `release`, `dist`, `package`) um
manifest de distribuição rastreado; isso não exclui outras convenções ou
configuração externa. Os módulos de referência e telemetria são separados.

## Necessário antes de concluir

1. Mantenedor: identificar quais fontes, crates, binários, exemplos, testes,
   imagens, fixtures, plataformas e features são candidatos reais.
2. Para cada candidato, gerar lista e SBOM a partir do build/revisão final,
   identificar código incorporado e bibliotecas empacotadas, e revisar textos
   integrais de licença, copyright e notices correspondentes. Resolver a
   divergência `modernc` sem substituir automaticamente evidência por conclusão.
3. Validar empacotamento e testes em ambiente isolado sem checkout adjacente,
   após definir a política de inclusão das fixtures e dependências locais.
4. Confirmar direitos de fixtures e contribuições, concluir revisão de notices
   e obter autorização explícita antes de licença final, release, tag ou push.
   Nenhum desses passos foi aprovado por esta inspeção.
