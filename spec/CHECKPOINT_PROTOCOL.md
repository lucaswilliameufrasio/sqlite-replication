# Protocolo de captura e checkpoint — CP-01 pendente

> **Estado: bloqueado / não aprovado.** A captura, o spool durável e a
> recuperação têm implementação interna e evidência local, mas não estão ligados
> à API pública e não executam checkpoint. Esta evidência parcial não autoriza
> alegar cobertura incremental saudável nem habilitar M2.

## Invariante a provar

Antes de uma operação poder reiniciar, truncar ou reutilizar WAL, todo commit
elegível precisa estar copiado, validado e durável em staging/spool. Nenhuma
janela entre liberação e nova fixação pode perder commits, trocar gerações ou
aceitar bytes de um ciclo de salts diferente. Se a prova/condição falhar, não
checkpointar e não reportar cobertura incremental saudável.

## Candidato de implementação

O desenho a validar usa adapter SQLite homologado, conexões gerenciadas e
políticas de conexão explícitas. O leitor fixa uma transação de leitura que
delimita snapshot/frames; o parser próprio valida header, checksum, salts e
commits; o encoder publica cada LTX completo e durável antes de liberar a
proteção. A rotação/checkpoint é solicitada somente sob locks e precondições
demonstrados. A leitura fixa, isoladamente, **não** é a prova.

Autocheckpoint é desligado em todas as conexões gerenciadas pelo adapter. As
conexões de teste também usam `busy_timeout=0`. Checkpoint continua sendo uma
operação externa/experimental: a captura não o solicita. Uma conexão não
gerenciada, driver diferente, VFS diferente, lock incompatível, erro real de
write/fsync ou estado WAL inesperado ainda precisa bloquear qualquer operação
destrutiva. `wal_hook` é apenas sinalização, nunca fonte única de verdade.

Cancelamento da captura é cooperativo: Go verifica o `context.Context` e Rust
usa um token atômico interno entre passos do Online Backup e limites de
publicação; uma syscall já em andamento não é preemptada. Antes da promoção
durável do watermark, cancelamento retorna erro, pode deixar LTX imutáveis
parciais e não avança o watermark; uma tentativa posterior reconcilia a cadeia.
Depois que o watermark e seu diretório foram sincronizados, a operação é
considerada concluída mesmo se o cancelamento chegar nessa fronteira final.

## Evidência local — parcial, não aprova CP-01

No Linux desta execução, os testes exercitaram Go/modernc SQLite **3.54.1** e
Rust/rusqlite bundled SQLite **3.53.2**, em filesystem local. O código exercitado
inclui `go/internal/capture`, `go/internal/walcapture`, `go/internal/spool`,
`go/internal/sqliteadapter` e os módulos Rust `core::capture`, `wal`, `spool` e
`sqlite-adapter`. `CaptureOnce` permanece interno e não executa checkpoint.

| Cenário coberto | Go | Rust | Escopo observado |
| --- | --- | --- | --- |
| `BEGIN IMMEDIATE` + reader pin; writer separado tenta iniciar | passou | passou | Writer bloqueado durante a lease; após sua liberação, writer separado consegue confirmar. |
| Barreiras determinísticas entre leitura, staging, publicação, liberação e nova captura | passou | passou | Writers recebem `SQLITE_BUSY` após leitura WAL, snapshot, tradução, publicação e no intervalo exato entre liberar o reader pin e o writer guard; `TRUNCATE` não completa nesse intervalo; watermark TXID 2 existe antes do checkpoint; após liberar o guard, novo commit é confirmado, truncado e reconciliado na próxima captura como TXID 3 (`WALOffset=0`). |
| `NOOP`, `PASSIVE`, `FULL`, `RESTART` e `TRUNCATE` durante guard/pin | passou | passou | `NOOP` é observado; os outros modos não modificam os bytes enquanto a lease está ativa. |
| `PASSIVE`, `FULL`, `RESTART` e `TRUNCATE` com reader antigo pinado, após novo commit | passou | passou | `PASSIVE` pode retornar `busy=0`, mas conta incompleta; os modos bloqueantes reportam `busy`/contagem incompleta. Nenhum remove o WAL do reader; depois de liberar o reader, `TRUNCATE` completa. |
| Tradução de WAL produzido pelo runtime real | passou | passou | Checksums/página lógica comparados a Online Backup do SQLite. |
| Vários commits, páginas repetidas, crescimento/redução, salts, prefixo truncado/alterado e cauda parcial | passou | passou | Limites commitados preservados; fronteira estrita rejeita prefixo de recuperação inválido. |
| `TRUNCATE` entre ciclos e reconciliação por imagem consistente | passou | passou | Commit já checkpointado entra como delta sem falsa proveniência WAL. |
| Ordem `stage sync → LTX imutável → watermark atômico`; retry e chain recovery | passou | passou | Falha após uma publicação de várias não salta os commits restantes. |
| Erros injetados nos limites de write/fsync | passou | passou | Hooks simulam erros ENOSPC/EIO; não provocam falha real de write/fsync. ENOSPC real em tmpfs está na linha seguinte. |
| ENOSPC real durante publicação parcial e recuperação | passou | passou | `make test-cp01-enospc`, Linux amd64, container rootless e tmpfs limitado a 8 MiB: falta de espaço durante a publicação, watermark não avança sobre a cadeia parcial, lease é liberada e nova captura recupera o WAL após liberar espaço. Não prova comportamento de filesystems de produção nem falha física de `fsync`. |
| Cancelamento cooperativo antes e durante publicação | passou | passou | Testes cancelam após o primeiro LTX e após o `fsync` do watermark staged, antes da promoção: retornam `context.Canceled`/`Interrupted`, conservam o watermark anterior, liberam a lease e a próxima captura reconcilia a cadeia. Cancelamento após watermark+diretório duráveis retorna sucesso. Syscalls em andamento não são preemptadas. |
| Conexão de aplicação não gerenciada, busy timeout e autocheckpoint | passou | passou | Conexão separada com `busy_timeout=250ms` e `wal_autocheckpoint=1` recebe `SQLITE_BUSY` durante a lease; depois da captura confirma, checkpointa/trunca e a captura seguinte reconcilia o commit como delta sem proveniência WAL. Limita-se ao runtime/driver desta linha. |
| `SQLITE_BUSY` na lease sem mutação do watermark e retry | passou | passou | Com watermark existente, uma lease ocupada retorna `SQLITE_BUSY` sem alterar TXID/checksum; após liberar o writer, uma nova captura reabre as conexões e publica o commit pendente. |
| Crash de processo durante checkpoint real e recuperação | passou | passou | Linux amd64: durante `PRAGMA wal_checkpoint(FULL)`, o pai envia `SIGKILL` após write bem-sucedido e, separadamente, após `fsync`/`fdatasync` bem-sucedido no arquivo principal; reabertura, `integrity_check`, 512 linhas e checkpoint `TRUNCATE` final passam. Cada limite repetido cinco vezes por engine. Não simula perda de energia/cache do SO. |
| Restauração independente de cadeias LTX capturadas | passou | passou | Linux amd64: o conformance gera dois commits capturados por engine, aplica snapshot+deltas com o CLI upstream `superfly/ltx v0.4.0`, verifica checksums LTX e compara integridade e todas as linhas restauradas com a origem. Repetir na matriz de plataformas/filesystems homologada. |
| Processo filho morto com `SIGKILL` nos limites de stage, publicação e watermark | passou | passou | No Linux, restart recupera sem perder commit nem duplicar LTX já publicado. |

O gate local `make test` passou nesta execução: formatação, `go vet`, Clippy com
warnings negados, suites Go/Rust, conformance Go↔Rust, restauração independente
de cadeias capturadas com `superfly/ltx v0.4.0` e smoke Floci isolado. Os
diretórios temporários da execução foram colocados em
Btrfs local (`/mnt/oioi`), em Linux amd64. Os testes de captura foram repetidos
25 vezes em Go e 20 vezes em Rust; a barreira reader-pin/writer-guard foi
repetida 20 vezes em cada engine. Isso não equivale à matriz de produção abaixo.
O drill opt-in `make test-cp01-enospc` também passou em ambos os engines no mesmo
host, provocando ENOSPC real em tmpfs de 8 MiB e verificando publicação parcial,
watermark sem avanço e recuperação após liberar espaço.
Os testes de `SIGKILL` são testes de processo em Linux; não são teste de perda
de energia nem de comportamento do filesystem após falha física.

Na implementação SQLite 3.51.3 consultada, `SQLITE_CHECKPOINT_NOOP` não toma o
checkpoint lock nem o writer lock: o fluxo pula a aquisição desses locks e lê o
wal-index. Portanto NOOP é uma observação, não uma barreira de sincronização.
Referência: [`wal.c`, `sqlite3WalCheckpoint()`](https://github.com/sqlite/sqlite/blob/version-3.51.3/src/wal.c#L4314-L4394).

## Matriz CP-01 ainda obrigatória

Use barreiras determinísticas/test hooks para pausar exatamente em cada
transição; `sleep` ou stress probabilístico sozinho não é prova.

1. Writer confirma entre leitura do limite e staging; verificar que o commit
   fica no LTX ou continua protegido para a próxima leitura em cada adapter/VFS.
   **Linux amd64 Go/Rust:** a barreira determinística agora prova que o writer
   fica bloqueado em cada fase e que o commit posterior, mesmo checkpointado
   antes da próxima captura, é reconciliado. Repetir nas demais combinações.
2. Writer confirma imediatamente antes/depois da liberação do read pin e antes
   da nova fixação; não há janela perdida. **Linux amd64 Go/Rust:** barreira
   determinística entre liberar o reader pin e o writer guard confirma `BUSY`;
   commit após liberação completa e antes da próxima captura, seguido de
   `TRUNCATE`, é reconciliado. Repetir nas demais combinações.
3. Checkpointer tenta `PASSIVE`, `FULL`, `RESTART` e `TRUNCATE` com readers
   longos e writers ativos; resultados `BUSY`/contagens incompletas nunca são
   tratados como sucesso. **Linux amd64 Go/Rust:** testes sob `BEGIN IMMEDIATE`
   seguem passando; novos testes com reader antigo pinado após novo commit
   confirmam contagem incompleta nos quatro modos (`PASSIVE` pode retornar
   `busy=0`) e que `TRUNCATE` só completa após liberar o reader. Repetir nas
   demais combinações.
4. Reset/reuse do WAL, salts/checkpoint sequence alterados, cauda incompleta,
   arquivo truncado e alteração concorrente durante read são detectados. Os
   testes determinísticos novos em Go e Rust passam localmente em Linux amd64:
   truncamento depois do `open`, aparecimento/desaparecimento entre verificações,
   troca atômica por outro arquivo de mesmo tamanho/`mtime` e sobrescrita
   in-place de mesmo tamanho com `mtime` restaurado são rejeitados. Go compara
   `os.SameFile` e `ctime` no intervalo de leitura em Linux/macOS; Rust compara
   device/inode e `ctime` durante a leitura em Unix. Depois da observação
   SQLite, a checagem final revalida identidade do caminho. Go também compilou
   os testes para Linux arm64, Darwin arm64 e Windows amd64; isso não é execução
   nativa. Escritas externas continuam sem coordenação pelo lock SQLite, e a
   prova não garante detecção de toda corrida/filesystem. A matriz arm64/macOS
   ainda precisa ser repetida com este código.
5. `SIGKILL` nos pontos antes/depois de stage fsync, publicação e watermark foi
   exercitado nos runtimes/OS locais. **Linux amd64 Go/Rust:** teste de processo
   filho executa `PRAGMA wal_checkpoint(FULL)` real; o pai observa syscalls e
   envia `SIGKILL` separadamente após write e após `fsync`/`fdatasync`
   bem-sucedidos no arquivo principal. Cada limite passou cinco repetições por
   engine. Após reabrir, `integrity_check`, todas as 512 linhas e um `TRUNCATE`
   completo passam. A captura continua sem solicitar checkpoint; isso não simula
   perda de energia nem prova persistência física. O conformance local também
   aplica, com o CLI upstream `superfly/ltx v0.4.0`, cadeias de dois deltas
   geradas pelos dois engines e verifica integridade/igualdade de linhas contra
   a origem. Repetir ambos os testes em cada combinação OS/filesystem/VFS
   homologada.
6. Hooks simulam falhas de write/fsync; o drill agora provoca `ENOSPC` real em
   tmpfs limitado no Linux amd64 e verifica recuperação, mas ainda faltam falhas
   físicas de `fsync` e interrupção de syscalls em andamento. Uma conexão
   externa de aplicação com timeout/autocheckpoint passou nos dois engines;
   ainda faltam drivers/VFS alternativos e outras configurações de conexão. O
   retry após `SQLITE_BUSY` na aquisição da lease, sem mudança do watermark e
   com captura posterior usando novas conexões, passou nos dois engines. O
   cancelamento cooperativo entre fases também passou nos dois engines.
7. Rodar para cada runtime SQLite/driver/OS explicitamente homologado e
   verificar banco restaurado contra os commits de origem.

### Resultado atual da matriz

| Plataforma da decisão D-16 | Resultado nesta execução |
| --- | --- |
| Linux amd64 — produção inicial | Testado em Btrfs local; Go `modernc.org/sqlite` 3.54.1 e Rust `rusqlite` bundled SQLite 3.53.2. Quality gate `make test` e drill ENOSPC em tmpfs passaram; CP-01 segue bloqueado. |
| Linux arm64 — produção inicial | `make test-native` passou em runner nativo `ubuntu-24.04-arm`, despacho manual 40114086587, commit `e2a1bc8`; o drill ENOSPC continua limitado a Linux amd64. Esse resultado antecede as alterações locais sem commit desta execução. |
| macOS arm64 — integração | `make test-native` passou em runner nativo `macos-15`, despacho manual 40114086587, commit `e2a1bc8`. Esse resultado antecede as alterações locais sem commit desta execução. |
| Windows 11 amd64 — fase posterior | Fora da primeira matriz D-16; não verificado. A validação de identidade/`ctime` no adapter Rust é Unix-only e a validação de `ctime` em Go é Linux/macOS-only; ambos falham fechados fora do escopo. |

Permanecem sem prova: drivers/VFS alternativos e conexões externas além da
configuração de aplicação testada; autocheckpoint global da aplicação;
escrita direta ou concorrente no sidecar fora do lock SQLite; falha física de
`fsync`/energia e ENOSPC em filesystems de produção (a evidência de espaço
cheio é somente em tmpfs);
interrupção de syscalls em andamento; repetição do crash de checkpoint além de
Linux amd64; falha de energia/VFS; e a matriz ampliada de drivers/VFS/filesystems.
Os testes nativos arm64/macOS passaram em revisão anterior, mas não cobrem estas
alterações locais. Os hooks provam ordenação e recuperação em pontos controlados,
não atomicidade de hardware ou durabilidade de todos os filesystems. O gate
segue bloqueado até haver evidência completa e revisão do protocolo.

## Critério para aprovação

Registrar sequência de locks/transações, versões, combinação driver/VFS/OS,
hooks/PRAGMAs, código exercitado, seed/carga, pontos de injeção e resultado de
cada caso. Exigir zero commits elegíveis perdidos, nenhum falso sucesso de
checkpoint e suites determinísticas repetidas em toda combinação homologada. A
aprovação deve ser revisada antes de conectar o protocolo à API pública. Até
lá, CP-01 permanece reprovado por ausência de evidência completa; M2 e promessas
de streaming permanecem bloqueadas.
