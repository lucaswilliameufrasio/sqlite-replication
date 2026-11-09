# Litestream: preparação da comparação local

Em 2026-11-05 foi verificada a viabilidade de um ensaio local de snapshot e
restore, não uma comparação de desempenho concluída. O binário oficial
Litestream **0.5.17** para Linux x86-64 foi obtido do release; o SHA-256 do
arquivo `litestream-0.5.17-linux-x86_64.tar.gz` foi
`cfb371176d164437ae869f8351cfde49bd1804ae71c61923f75c9cba9c9c006d`.

Em container rootless sem rede, com filesystem temporário, CPU limitada a 2 e
memória a 1 GiB, um banco SQLite sintético com 1.000 linhas foi submetido a:

```sh
litestream replicate -once -force-snapshot /work/source.db file:///work/replica
litestream restore -o /work/restored.db file:///work/replica
```

O comando de replicação reportou `one-shot replication complete`; o restore
abriu com `PRAGMA integrity_check = ok` e retornou `count(*) = 1000` e
`sum(id) = 499500`. Trata-se apenas de smoke: não foram medidas durações, nem
comparados todos os campos ou executadas repetições estatísticas.

O CLI Go de conformance oferece `snapshot DATABASE REPOSITORY` e
`restore REPOSITORY DATABASE`; o Litestream tem processo próprio de replicação
contínua, mas `-once -force-snapshot` oferece uma operação local pontual para
um experimento exploratório. Os formatos, versões de SQLite, inicialização,
durabilidade e operações executadas diferem; o mesmo resultado lógico não prova
equivalência de semântica ou interoperabilidade LTX. A versão Litestream 0.5.x
não é oráculo para o perfil LTX v2 deste projeto.

## Campanha exploratória local

O harness `conformance/benchmark-snapshot.py` usa os três binários informados
explicitamente, cria um banco WAL sintético determinístico e cópias e repositórios
independentes para cada tentativa. Cronometra processos completos de snapshot e
restore separadamente, inclui inicialização do processo e alterna a ordem. Após
cada restore verifica `integrity_check` e um SHA-256 calculado sobre todas as
linhas e payloads ordenados. Cada comando tem timeout de 120 s; falhas abortam a
campanha e o arquivo de saída não é criado. O arquivo de saída deve ser novo.

Exemplo, com binários previamente compilados e Litestream obtido e verificado:

```sh
python3 conformance/benchmark-snapshot.py --go /path/go-conformance \
  --rust /path/replication-conformance --litestream /path/litestream \
  --output /path/new-result.json --rows 10000 --warmups 1 --samples 5
```

Para comparar em isolamento, execute o harness num container sem rede e sem
credenciais, com somente um diretório de trabalho temporário gravável. O comando
não instala nem compila binários. Os CLIs Go e Rust desta campanha foram
compilados respectivamente com Go 1.27.3 (`go build`) e Rust 1.99.1
(`cargo build --release`), a partir de `f5c0984`; as alterações locais pendentes
incluíam somente documentação e um teste Rust, não código de produção. Litestream
0.5.17 foi o binário oficial mencionado acima. Hashes SHA-256 dos binários: Go
`52dcacfe5758e31d7b646b1f56b6c2616f2e760c7b6b17f4251a93e0f327f9a2`, Rust
`761fed450083963dcf64541d2ba942ef247ddff80d9cf520048ce13915239d64`, Litestream
`200e4248a4cc83da2ca52babe66774f1a71afa993d31373adb0ce0fe12154647`.

Em Linux x86-64, kernel 7.3.0, container rootless sem rede limitado a 2 CPUs e
2 GiB, filesystem local em `/home` e caches quentes não controlados, foram
executados um aquecimento e cinco amostras por implementação. O banco tinha
10.000 linhas com payloads sintéticos de 256 bytes, 2.748.416 bytes em disco;
Python usou SQLite 3.54.0. Todas as 18 tentativas, inclusive aquecimento,
passaram na verificação. Resultados brutos: `litestream-benchmark-2026-11-05.json`.

| Implementação | Mediana snapshot | Mediana restore | Bytes no repositório |
|---|---:|---:|---:|
| Go | 50,611 ms | 130,241 ms | 2.707.777 |
| Rust | 114,080 ms | 27,602 ms | 2.705.091 |
| Litestream | 87,833 ms | 41,809 ms | 5.398.226 |

Uma amostra de snapshot Litestream levou 456,164 ms e foi mantida nos dados.
O primeiro ensaio abortou antes de gerar resultado porque o banco estava em
modo DELETE; após definir WAL, a campanha completa acima foi reiniciada. O
filesystem e o host eram compartilhados e não foram medidos CPU/RSS, variação
de carga ou cache frio. As operações internas, SQLite usados pelos binários,
formatos e garantias diferem: não interpretar medianas ou tamanhos como ranking
equivalente, capacidade de replicação contínua, compatibilidade S3/R2 ou
desempenho de produção. M6 permanece aberto.
