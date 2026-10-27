# Plano de Desenvolvimento — SQLite Replication para Go e Rust

> **Versão:** 1.0 — 21/10/2026  
> **Estado em 2026-10-26:** M0 parcial, M1 verificado, M2 bloqueado por CP-01, M3 implementado e verificado localmente; M4–M6 incompletos.
> **Natureza:** duas bibliotecas independentes, open source, embarcadas no processo da aplicação  
> **Testes S3:** Floci local, sem homologação obrigatória em AWS S3 ou Cloudflare R2  
> **Nome de produto:** propositalmente indefinido; identificadores de código são provisórios.

## 1. Objetivo e definição do produto

Construir **dois motores próprios de backup/replicação assíncrona de SQLite**: um em Go e outro em Rust, com APIs idiomáticas para incorporação no ciclo de vida de aplicações. Os motores devem preservar alterações confirmadas no SQLite em arquivos imutáveis LTX, transferi-los para filesystem ou armazenamento S3-compatible e conseguir reconstruir bancos consistentes em momentos de recuperação selecionados.

**A proposta não é construir um SQLite distribuído.** É um produto de *disaster recovery* (DR) com um único escritor lógico por banco, restauração verificável, observabilidade e retenção. Não há sidecar, servidor separado, processo auxiliar permanente, RPC obrigatório ou dependência de Litestream na aplicação.

### Resultados esperados

- A aplicação inicia o replicador pela biblioteca e fecha-o de maneira controlada.
- Backups são gerados e persistidos **sem interromper transações normais** fora das seções críticas estritamente necessárias à segurança do WAL.
- Um host pode ser perdido; outro restaura **até o último ponto confirmado no destino**, não necessariamente até o último COMMIT local.
- Go e Rust podem produzir e consumir backups um do outro conforme a mesma versão de formato e layout declarados.
- Floci é o único emulador S3 obrigatório nos testes automatizados; filesystem é testado diretamente.
- A API expõe claramente RPO observado, erros, atrasos, limites de disco e impossibilidade de garantir segurança.

### Fora do escopo da v1

- Consenso, multiwriter, resolução de conflitos, replicação síncrona, leitura distribuída, eleição automática entre hosts e failover transparente.
- Compatibilidade prometida com qualquer driver SQLite/VFS ou filesystem de rede.
- Restauração automática de um banco **existente** e potencialmente corrompido sem autorização explícita.
- Criptografia proprietária inventada; quando habilitada, usar primitivas criptográficas estabelecidas, com envelope e versionamento.
- Integração de aplicação Node/SvelteKit: utilizar `sqlite-js-stream` da Platformatic separadamente; executar testes de interoperabilidade de formato quando possível, sem fazê-la dependência das bibliotecas.
- Qualquer teste obrigatório contra serviços pagos/reais; suporte a R2/AWS é via configuração S3-compatible e não implica homologação desses provedores.

## 2. Decisões arquiteturais — fechadas

| ID | Decisão | Consequência |
|---|---|---|
| D-01 | Go e Rust usam **implementações próprias** do motor | Sem reutilização do código Litestream, `literstream` ou wrappers de replicação |
| D-02 | Monorepo de desenvolvimento, publicação independente | Go module e crate Cargo autônomos; nenhum código de runtime compartilhado |
| D-03 | Especificação e fixtures compartilhadas | Interoperabilidade por contrato, não via FFI ou port da outra linguagem |
| D-04 | SQLite em modo WAL; um escritor lógico por banco | Somente DR; nenhuma promessa de escrita concorrente entre hosts |
| D-05 | Snapshot inicial consistente e LTX incremental | Reconstrução com cadeia verificável |
| D-06 | LTX **v2 como perfil de interoperabilidade inicial** | Seguir formato documentado pelo Litestream; v3 como trabalho isolado, após matriz de compatibilidade |
| D-07 | Versão do wire format declarada fora do arquivo | `LTX1` não distingue v2/v3; não inferir formato por heurística |
| D-08 | Storage v1: filesystem + adaptador S3-compatible | S3 SDK genérico; configuração de endpoint, região, credenciais e path-style |
| D-09 | Floci é o serviço S3 de testes e CI | Sem MinIO/RustFS e sem validação externa compulsória |
| D-10 | Arquivos LTX imutáveis e staging local durável | Publicar apenas objetos completos; retry idempotente |
| D-11 | Restauração offline, em arquivo temporário e com troca atômica | Nunca escrever em DB ativo ou sobrescrever DB existente por conveniência |
| D-12 | Sem criação de um formato próprio para substituir LTX | Metadados externos podem complementar layout/versão sem modificar bytes LTX |
| D-13 | Checkpoint sob política coordenada pelo motor | Não assumir que fazer `tail` do `-wal` é seguro |
| D-14 | OpenTelemetry + métricas Prometheus | Sem servidor HTTP obrigatório; registrador opcional da aplicação |
| D-15 | Licença Apache-2.0 proposta | Verificar atribuições e licenças de fixtures/referências antes da publicação |
| D-16 | Linux amd64/arm64 como primeira matriz de produção | macOS arm64 na integração; Windows 11 amd64 como fase posterior |
| D-17 | Segurança de dados tem prioridade sobre latência | Falhas explícitas e bloqueio de promoção quando um invariante não for comprovado |

**Nota sobre formatos:** a documentação do Litestream descreve arquivos LTX v2; o projeto `superfly/ltx` documenta v3 com quadros de página e compressão diferentes, embora ambos usem `LTX1`. Para a v1, os leitores e escritores **v2** precisam interoperar com fixtures reconhecidas da versão de Litestream testada. Suporte de escrita v3 **não** será misturado ao mesmo prefixo e não será anunciado como compatível com leitores v2.

## 3. Invariantes não negociáveis

**I-01 — Commit válido:** nunca gerar LTX contendo parcialmente uma transação WAL. Um frame WAL só finaliza a transação quando registra o tamanho de banco confirmado; frames incompletos, não confirmados ou com checksum/salt inválidos são ignorados ou causam erro diagnosticável.

**I-02 — Captura antes do descarte:** todo WAL que será reiniciado/truncado precisa ter seus commits elegíveis capturados e tornados duráveis no spool antes da operação. A segurança do protocolo de checkpoint é um *quality gate*, não uma suposição.

**I-03 — Cadeia recuperável:** não publicar como recuperável uma sequência com lacuna de TXID, checksum inconsistente ou snapshot-base ausente.

**I-04 — Imutabilidade:** arquivo confirmado no destino nunca é sobrescrito. Reenvio da mesma chave é idempotente somente se conteúdo e checksum forem idênticos; divergência é erro crítico.

**I-05 — Recuperação isolada:** recuperação escreve em destino temporário inacessível à aplicação e só promove para o caminho final após validação SQLite e sincronização dos metadados de filesystem.

**I-06 — Proveniência:** versão do LTX, geração/identidade lógica do banco, número de página e outros metadados necessários à restauração são conhecidos, versionados e consistentes.

**I-07 — Falhas explícitas:** nenhuma queda silenciosa de frames, upload ignorado, checkpoint inseguro ou truncamento de histórico sem aviso.

**I-08 — Sem falsa equivalência de RPO:** `último commit local`, `último arquivo LTX em disco` e `último TXID confirmado remotamente` são três posições distintas.

**I-09 — Fonte SQLite válida:** não tratar `-shm` como backup portátil; arquivos SQLite vivos não são copiados via `cp` como alternativa ao Online Backup API.

**I-10 — Bancos distintos:** `ATTACH` pode envolver transações que não são atomicamente replicáveis entre bases; não alegar consistência global de múltiplos bancos.

## 4. Desenho da solução

```text
┌─────────────────────────────────────────────────────────────┐
│                  Aplicação Go ou Rust                       │
│  HTTP / workers / consultas / transações SQLite             │
│                           │                                 │
│                  DB Adapter controlado                      │
│                           │                                 │
│                SQLite DB + WAL local                        │
│                           │                                 │
│       ┌───────────────────▼──────────────────────┐          │
│       │ Replicator embarcado                     │          │
│       │  [bootstrap e snapshot consistente]      │          │
│       │  [WAL reader e controle de checkpoint]   │          │
│       │  [LTX codec e cadeia de checksums]       │          │
│       │  [spool durável / fila de upload]        │          │
│       │  [compactação / retenção / métricas]     │          │
│       └────────────────────┬─────────────────────┘          │
└────────────────────────────┼────────────────────────────────┘
                             │
                   Storage interface
                ┌────────────┴────────────┐
                │                         │
         filesystem local         S3-compatible
                                       │
                             Floci (desenvolv./CI)
                             endpoints externos (uso)

   Recuperação: Store → validar cadeia → reconstruir arquivo
                      → integrity_check → promover offline
```

### 4.1 Módulos internos obrigatórios

1. **`sqlite` / `dbadapter`:** abertura/inspeção, leitura consistente, política de conexões, verificações de versão e checkpoints.
2. **`wal`:** leitura binária de cabeçalho e frames, validação incremental de checksums, salts e limites de commit; detecção de ciclos/reset.
3. **`ltx`:** encode/decode, validação estrutural, checksums, formatos e compactação por páginas.
4. **`position`:** TXID, salt/offset/epoch, estado local e remoto, prevenção de bifurcação da cadeia.
5. **`spool`:** arquivo temporário, `fsync`, rename atômico, manifesto local, replay após crash e cota.
6. **`replicator`:** workers de sync, backpressure, retries, reconciliação, checkpoint e shutdown.
7. **`storage`:** interface de armazenamento, filesystem e S3; uploads imutáveis, listagem, download e metadados.
8. **`restore`:** planejamento de cadeia, verificação, reconstrução, recuperação por ponto e validação final.
9. **`retention`:** snapshots, compactação, proteção de referências e coleta de lixo segura.
10. **`telemetry`:** status/health, métricas, traces e logs estruturados, sem acoplamento a framework HTTP.

## 5. Organização do monorepo

```text
sqlite-replication/
├── README.md
├── LICENSE
├── CONTRIBUTING.md
├── SECURITY.md
├── Makefile / justfile
├── spec/
│   ├── WAL.md
│   ├── LTX_V2.md
│   ├── LTX_V3_COMPATIBILITY.md
│   ├── REPOSITORY_LAYOUT.md
│   ├── RECOVERY_PROTOCOL.md
│   ├── CHECKPOINT_PROTOCOL.md
│   ├── STORAGE_CONTRACT.md
│   └── INVARIANTS.md
├── go/
│   ├── go.mod
│   ├── replication.go
│   ├── internal/{sqlite,wal,ltx,position,spool,sync,restore,retention}/
│   ├── storage/{fs,s3}/
│   ├── telemetry/
│   └── examples/{chi,worker}/
├── rust/
│   ├── Cargo.toml
│   ├── crates/{core,sqlite,wal,ltx,storage-fs,storage-s3,telemetry}/
│   └── examples/{axum,worker}/
├── conformance/
│   ├── fixtures/{wal,ltx-v2,ltx-v3,sqlite,corrupted}/
│   ├── cases/{snapshot,incremental,restore,compaction,retention}/
│   ├── schemas/{manifest,status}/
│   └── runners/{go,rust}/
├── tests/
│   ├── integration/
│   ├── stress/
│   ├── chaos/
│   ├── cross-language/
│   └── benchmarks/
├── infra/
│   ├── compose.yaml
│   ├── floci-bootstrap/
│   └── toxiproxy/
├── docs/
│   ├── getting-started/{go,rust}.md
│   ├── configuration.md
│   ├── drivers-and-platforms.md
│   ├── operations.md
│   ├── recovery-runbook.md
│   ├── observability.md
│   ├── compatibility.md
│   ├── threat-model.md
│   └── adr/
└── .github/workflows/
    ├── go.yml
    ├── rust.yml
    ├── conformance.yml
    ├── chaos.yml
    └── release.yml
```

Cada linguagem possui sua própria lógica de encode/decode LTX, WAL e máquina de estados. O diretório `spec/` descreve comportamentos; `conformance/` contém **dados de teste e testes**, nunca biblioteca de runtime.

## 6. Contrato LTX e layout do repositório

### 6.1 Perfil v1: LTX v2 interoperável

- Implementar **do zero** leitor e escritor v2 em Go e Rust, com base em especificação pública e vetores independentes.
- Validar campos de header (100 bytes), trailer (16 bytes), endianness, flags, pages, arquivo truncado, CRC-64 ISO e rolling checksum por página. Fixar vetores de checksum no repositório.
- `MinTXID`, `MaxTXID`, checksum pré/pós, tamanho lógico, timestamp e salt/offset são tratados como campos de protocolo; não identificam necessariamente cada transação SQLite.
- Fazer fixtures de diferentes tamanhos de página (1024, 4096, 8192, 65536), bancos vazios, páginas removidas, transações gigantes e sequências compactadas.
- A matriz de compatibilidade especifica **versão exata** das ferramentas de referência; a expressão “compatível com Litestream” sozinha não constitui critério de aceite.
- Não importar `github.com/superfly/ltx` nem `github.com/benbjohnson/litestream` na distribuição Go, nem crates equivalentes de replicação em Rust. Referências podem ser executadas **somente** como oráculos de testes em ambiente de desenvolvimento.

### 6.2 Perfil v3: preparação sem misturar layouts

- Especificar o leitor de v3 separadamente; a escrita v3 é uma evolução posterior a v1, sem bloquear a entrega v2.
- Fixar `format_version` em `repository.json` sob prefixo de metadados separado, **e/ou** em configuração explícita de importação; nunca concluir v2/v3 só pelo magic `LTX1`.
- Não armazenar v2 e v3 sob a mesma cadeia sem migração explícita que reconstrua e verifique o banco.
- Não supor que Litestream com leitor v2 poderá restaurar v3.

### 6.3 Layout e identidade

- `repository_id`: UUID gerado na inicialização do prefixo, impede anexar backup de banco incompatível ao mesmo histórico.
- `database_id`: identificador estável atribuído à base, não inferido somente do filename.
- `epoch` / `generation`: identidade lógica de uma sequência reinicializada intencionalmente, nunca para ocultar lacunas.
- Layout S3 compatível com o perfil de referência: `<prefix>/<level em hexadecimal com 4 dígitos>/<min_txid>-<max_txid>.ltx`.
- Layout filesystem compatível com a referência: `<root>/ltx/<level decimal>/<arquivo>.ltx`.
- **Os layouts de filesystem e S3 não são cópias 1:1.** Adaptador/restore deve conhecer o layout por backend.
- Metadados adicionais próprios: `<prefix>/_meta/repository.json` (versionado), `<prefix>/_meta/observations/` (opcional); jamais depender exclusivamente de um manifest mutável para descobrir arquivos existentes.
- Objetos LTX nunca são regravados; compactação cria novos arquivos de nível maior e só remove originais depois da verificação e política de retenção.

### 6.4 Convenções de restauração

- TXID: limite **inclusivo**; compactados só podem ser aplicados quando seu `MaxTXID` cabe integralmente no alvo.
- Timestamp: limite **exclusivo** (`arquivo.timestamp < alvo`), documentado explicitamente para manter semântica do perfil de referência.
- A precisão de recuperação é a fronteira entre LTX preservados, não a de cada COMMIT SQLite, e pode piorar quando a retenção elimina intervalos finos.
- Nunca tentar aplicar parte de um arquivo LTX compactado para atingir um TXID intermediário.

## 7. Integração com SQLite e captura segura do WAL

### 7.1 Pré-condições de inicialização

1. Identificar DB canônico, inode/identidade do arquivo, permissões, filesystem local suportado e `journal_mode=WAL`.
2. Verificar `SELECT sqlite_version()` **no runtime utilizado pela aplicação**, exigindo SQLite >= **3.51.3** para produção (ou backport formalmente homologado, fora da primeira matriz).
3. Rejeitar acesso via NFS/SMB/network filesystem e VFS não homologado.
4. Conexões utilizadas pelo replicador precisam compartilhar semântica de locking com as conexões da aplicação. Não abrir duas implementações SQLite incompatíveis e supor segurança por estarem no mesmo processo.
5. Configurar `busy_timeout`, synchronous, WAL autocheckpoint e hooks **em todas as conexões** relevantes, via adapter/factory, não apenas um `PRAGMA` eventual no pool.
6. Exclusividade local: lock do processo de replicação por banco; escritor remoto concorrente no mesmo prefixo não é suportado na v1.
7. No bootstrap, criar snapshot consistente antes da primeira cadeia de delta considerada recuperável. Banco já existente requer leitura via SQLite Online Backup API ou mecanismo nativo comprovadamente equivalente.

### 7.2 Estratégia de implementação e prova de segurança

**Captura incremental:** parser próprio do arquivo WAL, validando cabeçalho de 32 bytes, frames de 24 bytes + páginas, checksum cumulativo, salt, endianness, marcas de commit, offset consumido e tamanho lógico do banco. Copiar apenas transações completamente confirmadas para staging durável. A marca de commit é tratada como fronteira indivisível.

**Proteção contra reset:** uma leitura transacional SQLite devidamente fixada (*pinned read transaction*) é uma peça da solução para evitar reuso não detectado de segmentos ainda necessários; ela **não**, sozinha, prova um protocolo completo de rotação/checkpoint. O ciclo de pin, ingestão, persistência, liberação, checkpoint e nova fixação exige projeto de sincronização que impeça lacunas sob escritas concorrentes.

**Checkpoint:** desabilitar autocheckpoint em todas as conexões gerenciadas, manter leitor de proteção e delegar rotação/checkpoint controlado ao componente SQLite. Executar `PASSIVE`, `FULL`, `RESTART` ou `TRUNCATE` **somente** após verificar as precondições de cada modo e sem concluir sucesso por simples ausência de erro de chamada. `SQLITE_BUSY`, conexões externas, tamanho excessivo de WAL, leitores longos e falhas no spool têm caminhos de segurança explícitos.

**Gate obrigatório CP-01:** produzir documento de prova operacional com sequência detalhada de locks/transações e executar testes determinísticos com operações concorrentes nas janelas entre liberação e nova fixação. Se não houver uma implementação segura para o adapter escolhido, **não liberar replicação WAL incremental**. Um primeiro incremento de snapshot consistente continua útil, mas não pode ser apresentado como streaming DR completo.

**Hooks:** `sqlite3_wal_hook()` é apenas sinalização para reduzir latência; não é única fonte de verdade, pode ser sobrescrito pelo autocheckpoint e exige instalação por conexão. Polling seguro do WAL deve funcionar mesmo sem hooks.

### 7.3 Integração Go

- API pública com `context.Context`, estado tipado, storage interface e inicialização de restore separada.
- Adapter inicial com `modernc.org/sqlite`, `database/sql` e conexão SQLite dedicada para coordenação; padrão sem CGO.
- Documentar e testar o risco de locking POSIX ao misturar implementações SQLite; **não** prometer suporte a `mattn/go-sqlite3` na v1 sem prova própria.
- Usar `sql.Conn` dedicada quando estado por conexão for necessário; `sql.DB` é pool e `PRAGMA` aplicado a uma conexão aleatória não resolve todas as conexões.
- Evitar abrir/fechar descritores de DB que possam interferir em locks POSIX sem validar semântica de versão/OS; teste especial para isso.

### 7.4 Integração Rust

- Adapter inicial com `rusqlite` e feature `backup` para snapshot; Tokio para agendamento e I/O de storage.
- O worker SQLite síncrono possui suas próprias conexões numa thread/actor apropriado (`spawn_blocking` ou thread dedicada) e não bloqueia o executor async.
- Expor APIs compatíveis com `tokio` sem afirmar que `rusqlite::Connection` é `Sync`.
- `sqlx`/outros drivers entram apenas depois da homologação de locks, PRAGMAs, versão do SQLite e lifecycle de conexões.
- Não introduzir FFI entre Rust e Go. Usar `libsqlite3` via binding SQLite convencional não viola independência do motor.

## 8. Máquina de estados e durabilidade

### 8.1 Estados públicos

`NEW → BOOTSTRAPPING → HEALTHY → DEGRADED → NEEDS_OPERATOR → STOPPING → STOPPED`, com `FATAL` para cadeia irrecuperável, banco/versão incompatível ou checkpoint inseguro detectado.

**Expor pelo menos:** `local_committed_boundary`, `local_staged_txid`, `remote_confirmed_txid`, `last_successful_upload_at`, `last_recoverable_time`, `replication_lag`, `wal_bytes`, `spool_bytes`, `recovery_capability`, `last_error`.

### 8.2 Fluxo durável

1. Ler até a última fronteira de commit WAL válida.
2. Serializar o LTX completo em arquivo temporário.
3. Validar arquivo gerado (incluindo checksum e encadeamento) e `fsync` do conteúdo.
4. Renomear para spool imutável e persistir diretório/metadados locais conforme plataforma.
5. Registrar `local_staged_txid`, sem avançar `remote_confirmed_txid`.
6. Enviar ao backend com chave imutável e, quando suportado, condição `If-None-Match: *`.
7. Após confirmação do backend, verificar o resultado e a identidade de conteúdo; persistir novo watermark remoto.
8. Retomar arquivos do spool no restart; objetos já existentes são aceitos apenas quando conteúdo e checksum coincidem.
9. Somente então liberar entradas locais cobertas pela política de retenção e pelas garantias de recuperação.

Em caso de queda entre upload e confirmação local, a reconciliação enumera objetos e revalida hashes; jamais presume que um retry pode sobrescrever conteúdo divergente.

### 8.3 Quota e backpressure

- `max_spool_bytes`, `max_wal_bytes` e `max_unprotected_age` configuráveis; valores padrão propostos: 1 GiB de spool, alerta em 70/85%, sincronização em 1 s e solicitação de rotação perto de 64 MiB de WAL capturado.
- Em indisponibilidade, manter backlog dentro de limites e distinguir writes locais ainda não replicados.
- **Modo estrito:** o host fornece integração de admissão de escritas; quando a segurança não pode ser mantida, a biblioteca solicita/provê `denyWrites` e expõe indisponibilidade. Se o adapter não consegue garantir esse bloqueio, rejeitar configuração estrita ao iniciar.
- **Modo explícito degradado:** continuar servindo escritas mesmo com proteção comprometida exige consentimento operacional explícito; readiness passa a degradado e RPO pode ficar ilimitado.
- Não afirmar que uma biblioteca consegue impedir escritas por conexões externas que não controla.

## 9. Storage e concorrência

### 9.1 Contrato de storage (ambas as linguagens)

`PutImmutable(key, stream, metadata)`, `Get(key, range?)`, `Head(key)`, `List(prefix, cursor)`, `DeleteBatch(keys)`, `Capabilities()`. Implementação pode acrescentar operações multipart, cancelamento e timeout. Métodos precisam ser testados com 0 bytes, objeto grande, interrupção, retries e resposta inesperada.

**Filesystem:** escrita temporária no mesmo volume, `fsync` arquivo + diretório, `rename` atômico, nomes seguros, tolerância a processos interrompidos.

**S3:** endpoint customizado, path-style, região, credenciais dinâmicas do provider chain, TLS, timeout, multipart, backoff com jitter, stream e checksum; não confiar em ETag como MD5 do conteúdo (multipart e implementações S3 podem divergir).

### 9.2 Concorrência de escritores

- Na v1: lock local exclusivo do replicador, identidade estável, uma instância autorizada e prefixo não compartilhado por escritores independentes.
- `If-None-Match` e checksums evitam **substituições de objetos**, mas não constituem lease distribuído e **não impedem split-brain** por si só.
- Se múltiplos hosts usarem o mesmo prefixo como escritores, iniciar deve falhar quando o conflito for detectável; impossibilidade de garantir exclusividade deve estar documentada. Leases/fencing via mecanismo confiável ficam para v2.

### 9.3 Retenção

- Proposta inicial: retenção histórica de 7 dias, pelo menos dois snapshots completos verificados e horizonte mínimo de recuperação fina configurável.
- O GC constrói grafo de referências dos snapshots e incrementais; nunca deleta o último caminho consistente até qualquer ponto ainda prometido.
- Compactação produz novo arquivo e o valida por restauração/checagem de cadeia antes de deletar fontes.
- `retention.dry_run()` e `explain_restore(target)` disponíveis para operação segura.

## 10. Backup inicial, restauração e recuperação

### 10.1 Snapshot inicial

- Copiar estado consistente usando SQLite Online Backup API para arquivo temporário (não `cp` de arquivo ativo).
- Registrar page size, checksum do estado inicial e metadados de origem, criar LTX snapshot inicial; publicar base antes de afirmar que a cadeia tem cobertura remota.
- Mudanças ocorridas durante bootstrap precisam continuar disponíveis pelo protocolo de captura protegido; não existe janela sem cobertura entre snapshot e deltas.

### 10.2 API de restauração

- Operação offline antes da abertura do pool de aplicação; `RestoreIfMissing` é **opt-in** e só atua se arquivo destino não existir.
- Alvos: último recuperável, TXID inclusivo, instante UTC inclusivo (maior TXID
  com timestamp LTX `<=` ao instante) ou snapshot explícito. Timestamp seleciona
  fronteiras LTX disponíveis, não commits intermediários.
- Descoberta de arquivos → plano de recuperação → validação da cadeia/checksums → arquivo temporário → aplicar páginas/truncamento lógico → `PRAGMA quick_check` e `integrity_check` → `fsync` → promoção atômica.
- Falha em qualquer passo preserva arquivo anterior e registra diagnóstico. Não aceitar DB final com `-wal`/`-shm` antigos e incompatíveis.
- `RestoreTo(path)` nunca abre o banco destino para escrita concorrente; arquivo de destino preexistente é rejeitado por padrão, com modo de substituição explícito e precondições fortes.
- Recuperação em outra máquina usando apenas artefatos remotos e configuração/credenciais documentadas.

### 10.3 Disaster recovery runbook

1. Interromper ingressos de novas escritas e isolar o host comprometido.
2. Inspecionar `repository_id`, último TXID íntegro e pontos recuperáveis.
3. Selecionar alvo, baixar e restaurar em diretório temporário.
4. Rodar checagens, comparar contagens/hashes de fixtures esperadas, validar consistência transacional.
5. Promover DB e iniciar aplicação apenas depois da verificação.
6. Iniciar nova cadeia de replicação conforme política explícita de ownership; nunca permitir dois hosts ativos no mesmo repositório.

## 11. API pública proposta

> Exemplos ilustram o contrato desejado; nomes, imports e assinaturas serão congelados após a primeira implementação vertical. Não são código de bibliotecas já existentes.

### 11.1 Go

```go
store, err := replica.NewS3Store(ctx, replica.S3Config{
    Endpoint: "http://localhost:4566",
    Bucket: "sqlite-backups",
    Prefix: "apps/demo",
    Region: "us-east-1",
    ForcePathStyle: true,
})
if err != nil { return err }

// Antes de abrir conexões da aplicação, se autorizado:
if err := replica.RestoreIfMissing(ctx, store, "./data/app.db"); err != nil {
    return err
}

appDB, err := openConfiguredSQLitePool("./data/app.db")
if err != nil { return err }

r, err := replica.Start(ctx, replica.Config{
    DatabasePath: "./data/app.db",
    Adapter: replica.ModerncAdapter(appDB),
    Store: store,
    SyncInterval: time.Second,
    Safety: replica.SafetyStrict,
    WriteGate: applicationWriteGate,
})
if err != nil { return err }

// Shutdown: parar entrada de writes -> r.Flush(ctx) -> appDB.Close() -> r.Stop(ctx).
status := r.Status()
_ = status
```

### 11.2 Rust

```rust
let store = S3Store::builder()
    .endpoint("http://localhost:4566")
    .bucket("sqlite-backups")
    .prefix("apps/demo")
    .region("us-east-1")
    .force_path_style(true)
    .build()
    .await?;

Restore::if_missing(&store, "./data/app.db").await?;
let adapter = SqliteAdapter::rusqlite("./data/app.db")?;
let replicator = Replicator::builder(adapter, store)
    .sync_interval(Duration::from_secs(1))
    .write_gate(app_write_gate)
    .start()
    .await?;

// Shutdown: fechar admissão -> flush -> fechar conexões de negócio -> stop.
replicator.flush().await?;
let status = replicator.status();
```

### 11.3 Operações públicas obrigatórias

- `Start`/`Stop`: idempotência documentada; erro se já houver replicador para o banco.
- `Sync`/`Flush`: modo local e remoto distinguíveis; `Flush(remote=true)` só retorna sucesso após TXID remoto confirmado.
- `Status` e `SubscribeEvents`: estados tipados, sem revelar credenciais.
- `RestoreIfMissing`, `RestoreTo`, `ListRestorePoints`, `ExplainRestore`.
- `VerifyRepository` e `VerifyRestore` (sem modificar banco ativo).
- `RetentionDryRun`/`RunRetention` com políticas explícitas.
- `Checkpoint` somente através do motor e sujeito a precondições.

## 12. Configuração e defaults propostos

| Configuração | Default inicial | Observação |
|---|---|---|
| `sync_interval` | 1 s | ajustável; não é garantia de RPO |
| `local_spool_limit` | 1 GiB | configuração obrigatória para produção |
| `max_wal_before_checkpoint_request` | 64 MiB | solicitação de rotação segura, não truncate forçado |
| `upload_concurrency` | 2 por banco | ordenar publicação lógica das posições |
| `retry` | exponential backoff + jitter | distinguir erros transitórios/permanentes |
| `retention` | 7 dias | proteger no mínimo dois snapshots íntegros |
| `snapshot_interval` | 24 h | não confundir com checkpoint |
| `safety` | estrito quando write gate disponível | sem write gate estrito, startup falha |
| `auto_restore_if_missing` | false | ativação explícita do integrador |
| `s3.path_style` | false | `true` na configuração Floci |
| `verify_remote_upload` | head + checksum quando suportado | não usar somente ETag como hash |
| `encryption_at_rest` | provider managed | E2E encryption opcional numa fase posterior |

Esses defaults serão confrontados com benchmarks e testes; o resultado final vira ADR. Nenhum limite implica durabilidade ilimitada durante indisponibilidade do storage.

## 13. Telemetria e diagnóstico

**Métricas (por DB com labels de baixa cardinalidade):**

- `sqlite_replica_remote_confirmed_txid` e `sqlite_replica_local_staged_txid` (ou gauges representáveis com segurança, considerando faixa de uint64).
- `sqlite_replica_replication_lag_seconds`, `sqlite_replica_last_upload_timestamp_seconds`.
- `sqlite_replica_wal_bytes`, `sqlite_replica_spool_bytes`, `sqlite_replica_uploaded_bytes_total`.
- `sqlite_replica_upload_errors_total{category}`, `sqlite_replica_retry_total`, `sqlite_replica_sync_duration_seconds`.
- `sqlite_replica_checkpoint_duration_seconds`, `sqlite_replica_checkpoint_busy_total`.
- `sqlite_replica_restore_duration_seconds`, `sqlite_replica_restore_failures_total`.
- `sqlite_replica_state` e `sqlite_replica_restore_points_available`.

**Logs:** JSON ou logger injetado (`slog` em Go; `tracing` em Rust); nunca incluir secrets, tokens, payloads SQL ou dados pessoais. **Traces:** spans de snapshot, sync, serialize, put, verify, compact e restore. **Health:** interface `Health()` retorna estado; a aplicação decide se publica `/health`, `/ready` ou exporta métricas. Nenhum listener obrigatório.

**Alertas sugeridos:** upload parado, RPO acima do objetivo operacional, spool > 70/85%, WAL sem checkpoint, checksum divergente, objeto ausente, dois escritores detectados e retenção bloqueada.

## 14. Testes — matriz de conformance

### 14.1 Unitários e propriedade

| Código | Cenário | Evidência exigida |
|---|---|---|
| UT-01 | Header WAL válido/inválido | parser idêntico em Go/Rust |
| UT-02 | Frames de commit/rollback | jamais serializar transação parcial |
| UT-03 | Checksums WAL e byte order | fixtures oficiais/independentes |
| UT-04 | LTX v2 header/trailer | golden bytes e cross-language |
| UT-05 | Checksum e corrupção de páginas | erro tipado, sem restore parcial |
| UT-06 | TXID range/overflow/gaps | rejeição inequívoca |
| UT-07 | Page sizes e crescimento/redução do DB | imagem restaurada íntegra |
| UT-08 | Fuzz parser WAL/LTX e manifesto | sem panic, OOM descontrolado ou UB |
| UT-09 | Serialização determinística | igualdade byte a byte com fixture |
| UT-10 | Estado do spool com interrupções | replay idempotente |

### 14.2 Integração SQLite

| Código | Cenário | Evidência exigida |
|---|---|---|
| IT-01 | Snapshot inicial com writes concorrentes | restauração consistente, sem janela perdida |
| IT-02 | WAL cresce durante sync | não mistura frames incompletos |
| IT-03 | Checkpoint automático externo | proteção ou bloqueio por config não suportada |
| IT-04 | Reader longo e checkpoint BUSY | sem perda, degradação visível |
| IT-05 | WAL reset e salts alterados | detecção sem salto de commits |
| IT-06 | Múltiplas conexões e transações extensas | sem bloqueio silencioso/perda |
| IT-07 | Queda SIGKILL em cada ponto do pipeline | último estado remoto permanece restaurável |
| IT-08 | `ENOSPC` no spool | bloqueio/degradação explícita |
| IT-09 | Shutdown com flush pendente | comportamento documentado |
| IT-10 | Restore enquanto aplicação ativa | falha segura, sem substituição |
| IT-11 | Driver/SQLite não homologado | rejeição descritiva |
| IT-12 | Reabertura após crash do host | estado reconciliado |

### 14.3 Integração S3 — apenas Floci

- `PutObject`, `GetObject`, `HeadObject`, `ListObjectsV2`, `DeleteObject`, operação com prefixos.
- Criar bucket no bootstrap do teste e testar tentativa de criar objeto existente com `If-None-Match` (esperado `412` quando o provedor dá suporte).
- Multipart: upload grande, interrupção, partes fora de ordem, retry, abort e limpeza.
- Persistência em modo `persistent`: desligar e iniciar **o mesmo** Floci mantendo volume, então restaurar dados.
- Erros 403/404/409/412/429/5xx, timeouts, truncamento de stream e latência usando Toxiproxy e/ou transport injetável.
- `ListObjectsV2` paginada, duplicidade eventual de resposta e ordenação não confiável tratadas no cliente.
- Conflito entre dois escritores simulado; verificar rejeição de sobreescrita, **sem** alegar prevenção distribuída de split-brain.
- Verificações de contrato S3 não implementadas pelo Floci devem ser registradas como limitação, nunca marcadas artificialmente como aprovadas.

### 14.4 Cruzados (não negociáveis)

- Go cria LTX v2 → Rust restaura e valida hash lógico do banco.
- Rust cria LTX v2 → Go restaura e valida hash lógico do banco.
- Go/Rust restauram fixtures v2 da versão exata de Litestream homologada.
- Ferramenta Litestream de referência restaura arquivo criado por Go/Rust, quando versão/layout correspondem; fixture/programa de referência apenas na CI de conformance, não nas dependências dos motores.
- Bancos com 0, 1, 100k, 1M de linhas; índices, BLOB, transações enormes, rollback, `VACUUM`, reinicialização do WAL e páginas reaproveitadas.
- Independência: construção de Go sem toolchain Rust e Rust sem toolchain Go; teste integrado compara artefatos por contratos.

## 15. Benchmarks e critérios quantitativos

**Capturar baseline da aplicação sem replicação** e executar mesmas cargas com replicação habilitada para cada linguagem.

| Perfil | Carga | Medidas |
|---|---|---|
| B-01 | 100 mil inserts pequenos | throughput, p50/p95/p99 commits, CPU/RSS |
| B-02 | 1 milhão de linhas | tempo de snapshot, upload, restore |
| B-03 | 70/30 leitura/escrita concorrente | contenção, checkpoints, latência |
| B-04 | BLOB de 1–16 MiB | alocação e throughput streaming |
| B-05 | transação única muito grande | limite de batch por commit e memória |
| B-06 | mudança de 1% em DB grande | bytes de delta vs snapshot |
| B-07 | storage lento e indisponível | spool, RPO real, adaptação |
| B-08 | 100% restauração após 1000 crashes injetados | ausência de inconsistência |
| B-09 | compaction/retention de cadeia longa | latência restore, IO e uso em disco |
| B-10 | confronto com Litestream de referência | mesma carga, HW, configuração e RPO |

**Qualidade mínima:**

- Zero restaurações silenciosamente corrompidas ou cadeias afirmadas válidas com lacunas nos conjuntos de testes.
- 100% dos checkpoints críticos exercitados com injeção de falhas e pelo menos um teste determinístico por transição de lock.
- Uso de memória não pode crescer linearmente com o tamanho do DB ou WAL apenas por serialização/transferência; buffers têm limites definidos.
- Metas iniciais sujeitas a validação: overhead p95 de write < 15% em carga B-01 comparado ao baseline, RPO típico < 5 s sem falhas e DB de referência; **não são SLAs prometidos**.
- Restaurar bancos de 1 GiB com RSS controlada por buffer; publicar medidas reais de CPU, RSS, tempo, I/O e dispersão (mínimo 5 execuções).
- Variância e hardware registrados; regressões > 15% em throughput/latência necessitam investigação ou ADR antes do release.

## 16. Infraestrutura de testes: Floci

Floci é pinado em `infra/compose.yaml` para `floci/floci:2.3.0` e digest OCI
index `sha256:e0b292df4dc9559b7b1bf4f51bdda4bb8e269276236438cd03cae661260ea8a9`;
os digests `linux/amd64` e `linux/arm64` estão registrados em
`infra/floci-image.lock`. Não usar `latest` em pipelines reproduzíveis.

`bash infra/floci-smoke.sh` (também exposto como `make test-floci`) foi
executado com Docker rootless e passou: sobe projeto/volume exclusivos,
aguarda health, assina requests S3 com credenciais sintéticas, cria bucket e
objeto, para/inicia o mesmo serviço, lê e compara os bytes persistidos, remove
objeto/bucket e derruba somente o projeto/volume desta execução. A porta host é
selecionada para a execução e publicada em loopback. Não foram usados AWS S3,
R2 ou credenciais reais. Evidência corresponde ao smoke do emulador, não a um
adapter de storage da biblioteca (ainda não implementado).

O ambiente inicial é `persistent`, que é a semântica requerida para este
smoke. Uma suite rápida em modo `memory` pode ser adicionada junto dos testes do
adapter S3; ela não substitui persistência/restart. Credenciais de teste são
`test`/`test`, região `us-east-1`, endpoint local e path-style.

**Sem validação externa:** não exigir contas AWS, R2 ou APIs de storage reais
para aprovar merge/release. A CI pode precisar baixar a imagem digest-pinned do
registry de containers se ela não estiver em cache; isso não utiliza serviço de
storage real. Explicitar que a fidelidade de Floci não prova semântica de todo
provedor S3-compatible; por isso os contratos do adapter incluem checagens de
capabilities e erros explícitos.

## 17. Roadmap incremental, entregas e gates

### Fase 0 — Contratos e infraestrutura [M0]

**Entregáveis:** monorepo, workflows, spec WAL/LTX, fixtures independentes, Floci versionado, ADRs, template de documentação, exemplos mínimos de SQLite. Criar primeiros testes de checksum, parser e persistência do Floci antes do motor completo.

**Gate M0:** CI reproduzível sem cloud real; baseline de fixtures v2 registrado; specs revisadas; ambiente sobe e derruba Floci consistentemente.

**Estado observado:** implementação e gate local `make test` verificados. Há
contratos WAL/LTX/layout/restore, snapshots LTX v2 independentes para páginas
512/1024/4096/8192/65536, vetores WAL para ambas as ordens de checksum e página
65536, parsers isolados Go/Rust com casos de corrupção/cauda, exemplos de
snapshot/restore em ambas as linguagens e smoke Floci persistente aprovado. M0
permanece parcial no escopo amplo: a matriz da seção 6 ainda não inclui todos
os casos de deltas/compactados e stress; aquela validação local ainda não tinha
observação de CI remoto. O incremento local de 2026-10-25 acrescentou bancos
sem tabelas e tabelas vazias nos cinco tamanhos de página, além de
1/100 mil/1 milhão de
linhas, BLOBs de 1/16 MiB, rollback e VACUUM/reutilização em páginas de 4096.
`make test` passou com 16 casos e 96 restores Go↔Rust↔oráculo v0.4.0, cada um
com schema/valores completos verificados por SHA-256 lógico e checks SQLite.
A matriz também rejeitou 32 restores com metadados inválidos e aceitou duas
restaurações com UUIDs válidos em maiúsculas. Os parsers Go/Rust agora concordam
na identidade UUID v4 hifenizada e variante RFC 4122. O caso de um milhão
excedeu 300 s nos restores Rust antes da otimização do CRC. Após a tabela CRC-64
ISO, a matriz final passou com o binário Rust release, sem remover o caso,
aumentar o timeout ou reduzir as verificações. As suites Rust de
unidade/integração continuam no perfil debug. Isso não é baseline de performance
nem prova de captura incremental nesses workloads.

### Fase 1 — Primeiro vertical: snapshot consistente → LTX → filesystem → restore [M1]

**Entregáveis:** adaptador SQLite suportado em cada linguagem, snapshot consistente via API SQLite, encoder/decoder LTX v2 independente, backend fs, restore offline e teste cruzado de snapshot. Construir Go e Rust em paralelo após contrato.

**Gate M1:** ambas as linguagens restauram snapshots mutuamente, passam `integrity_check` e hash lógico e rejeitam corrupção; sem prometer incremental nesse marco.

**Estado observado:** gate M1 verificado localmente por `make test`: snapshot,
restore cruzado Go↔Rust, verificação SQLite/hash lógico e fixtures do oráculo
v2 para cinco tamanhos de página; testes rejeitam corrupção e destino existente.

### Fase 2 — Incremental WAL e checkpoint seguro [M2 — caminho crítico]

**Entregáveis:** parser WAL, boundaries de commit, offsets/salts, read-pin, protocolo de checkpoint provado, staging durável, reinicialização após crash, verificação de versões SQLite e configuração por conexão.

**Gate M2:** CP-01 aprovado com testes concorrentes e de crash; nenhum WAL commit elegível perdido durante rotação. Se o gate não passar, **bloquear a conclusão da fase e produção**.

**Estado observado em 2026-10-25:** bloqueado por CP-01. Existe um protótipo
interno de captura/WAL e spool durável, ainda não exposto pela API pública. Em
Linux amd64, Go e Rust passaram testes de barreiras concorrentes, recuperação
após `SIGKILL`, publicação parcial com ENOSPC real em tmpfs limitado,
cancelamento cooperativo e reconciliação após escrita/checkpoint por conexão
externa de teste. A captura não executa checkpoint. Os novos testes locais
determinísticos também rejeitam truncamento após abrir o sidecar e substituição
atômica por arquivo de mesmo tamanho/mtime, além de aparecimento/desaparecimento
entre verificações e sobrescrita in-place de mesmo tamanho com `mtime` restaurado
(usando `ctime`); isso não prova segurança geral contra gravação direta
concorrente. Go também compilou os testes para Linux arm64, Darwin arm64 e
Windows amd64, sem executá-los nesses sistemas. Novos testes também
exercitam `PASSIVE`, `FULL`, `RESTART` e `TRUNCATE` atrás de reader antigo:
contagem incompleta nunca é tratada como checkpoint completo, e o `TRUNCATE`
só completa após liberar o reader. Ainda falta repetir a matriz nativa após
esta alteração. Os jobs nativos
`make test-native` para Linux arm64 e macOS arm64 passaram no despacho manual
40114086587, no commit `e2a1bc8`; isso não inclui o drill ENOSPC Linux amd64 nem
completa CP-01 nem cobre as alterações locais sem commit desta execução.
Permanecem pendentes falhas físicas de `fsync`/energia, repetição do crash de
checkpoint além de Linux amd64 e as demais combinações driver/VFS/filesystem.
Nos dois engines, teste Linux amd64 mata o processo durante um
`PRAGMA wal_checkpoint(FULL)` depois de write e, separadamente, de
`fsync`/`fdatasync` bem-sucedidos no arquivo principal; reabertura,
`integrity_check`, 512 linhas e `TRUNCATE` passam, cada limite repetido cinco
vezes por engine. Isso não prova perda de energia/cache do SO. Testes adicionais
confirmam que `SQLITE_BUSY` ao adquirir a lease não altera TXID/checksum do
watermark existente e que uma nova tentativa, após liberar o writer, reabre
conexões e captura o commit pendente. Ver
`spec/CHECKPOINT_PROTOCOL.md`. O conformance local também gera cadeias de dois
deltas em cada engine, aplica snapshot+deltas pelo CLI upstream
`superfly/ltx v0.4.0` e verifica integridade e igualdade linha a linha com a
origem; isso não substitui a matriz de plataforma/filesystem e não aprova CP-01.
**Progresso registrado em 2026-10-25:** `TMPDIR=/mnt/oioi make test`,
`TMPDIR=/mnt/oioi make test-cp01-enospc`, `go test ./... -count=1` e
`go mod tidy -diff` passaram em Linux amd64. Go compilou `sqliteadapter` e
`capture` para Linux arm64, Darwin arm64 e Windows amd64; isso não é execução
nativa. Gitleaks 8.30.1 não encontrou segredos na árvore de trabalho nem nos
três commits disponíveis. Esses resultados locais não substituem a matriz nativa
manual nem aprovam CP-01.

**Validação adicional deste incremento:** `make test` e
`make test-cp01-enospc` passaram novamente depois da otimização de CRC. A
implementação de CRC-64 ISO do
decoder Rust foi otimizada de oito passos de bits por byte para tabela de 256
entradas calculada em compilação; o check value padrão e o conformance LTX
cruzado passaram. Depois dessa correção, os 16 casos/96 restores da matriz
passaram com limite de 300 s. Os testes CP-01 Go/Rust também passaram em Btrfs
local: Go `internal/capture` e `internal/sqliteadapter` cinco vezes; Rust
`core::capture` e todos os testes `sqlite-adapter` uma vez. As primeiras
repetições do crash de checkpoint Go encontraram
`ESRCH`: instrumentação confirmou migração da thread do tracer, corrigida com
`runtime.LockOSThread` no pai. A nova regressão rastreia oito filhos em paralelo;
ela falhou antes da correção e passou, junto ao teste sequencial, em 50
execuções após a correção. Go compilou `storagefs`, `sqliteadapter` e `capture`
para Linux arm64, Darwin arm64 e Windows amd64; não houve execução nativa nesses
sistemas. As repetições CP-01 descritas acima usaram Btrfs local, e o drill
ENOSPC segue em tmpfs dedicado; isso não prova falha física de `fsync`/energia
nem substitui a matriz nativa/filesystem pendente. O gate
completo final inclui a matriz ampliada de snapshots descrita em M0 e o smoke
Floci; esses testes não aprovam CP-01. Os adapters S3 foram implementados e a
conformance M3 verificada posteriormente em Floci, sem homologar provedores
externos.

**CI observada em 2026-10-25:** o gate de push do commit `03f9b83` falhou no
checkpoint final concorrente (`busy=1`, contagens `-1`); a matriz manual passou
em Linux arm64 e falhou em macOS arm64 por `mapfile` incompatível com Bash 3.2.
O runner agora evita `mapfile` e `find -maxdepth`; o teste Go fecha a conexão de
verificação antes do `TRUNCATE` final. A matriz de snapshots passou novamente
isolada (16 casos/96 restores), o teste concorrente passou 100 repetições com
`GOMAXPROCS=2` e o `make test` completo passou localmente. As correções foram
publicadas no commit `72c834d`; a matriz manual pós-correção (run
`40172962620`) passou em Linux arm64 e macOS arm64, incluindo conformance. O
gate padrão Linux amd64 passou no run `40173262007` (commit `961583f`); a
atualização de evidências também passou no run `40266208888` (commit `e6ee152`).

**Atualização em 2026-10-26:** o drill opt-in QEMU de ext4/virtio-blk passou três
execuções para Go e Rust: após `SIGKILL` do processo QEMU e novo boot, ambas as
implementações recuperaram TXID/checksum e passaram recovery idempotente. Para
contornar o limite local de io_uring sem mudar configuração global, o QEMU rodou
numa unidade temporária `systemd-run --user` com seccomp restrito. A imagem raw
residia em tmpfs do host; isso não prova falha do host, mídia persistente ou
perda física de energia. **O gate CP-01 completo permanece pendente**, conforme
pedido do operador: não avançar para aprovação nem para exposição WAL/M2 enquanto
a matriz e a revisão restantes não forem concluídas. O trabalho independente
de storage M3 pode avançar em paralelo, conforme a seção 18.

### Próximos passos — CP-01 (atualizado em 2026-10-26)

1. **Concluído:** corrigir os gates reproduzíveis. `make test`, conformance
   (16 casos/96 restores), 100 repetições do cenário concorrente e os gates
   remotos Linux amd64, Linux arm64 e macOS arm64 passaram. O commit `826c8e6`
   também passou o quality gate remoto no run `40298291303`; a job da matriz
   nativa foi ignorada nesse evento push. Após o fixture FUSE, `make test` e
   `make test-cp01-fsync-interrupt` passaram localmente para Go/Rust. O código do
   slice em `0c977eb` passou Linux arm64 e macOS arm64 no run manual
   `40300871827`; o push gate do HEAD `4259009` passou no run `40300905561`.
   O run `40300698296` usava `826c8e6`, foi cancelado e não conta como evidência
   deste slice. Isso não aprova CP-01.
2. **Concluído — seam de erro no limite de I/O:** Go/Rust agora permitem, somente
   em funções internas de teste, fazer o sync de arquivo e o sync do diretório
   pós-rename retornarem EIO. `make test` passou: watermark não avança quando
   falha o sync do LTX ou do watermark staged; erro após rename pode deixar o
   watermark novo visível; retries repetidos e recovery de LTX publicado com
   watermark antigo são idempotentes. Isso injeta o retorno da operação na seam,
   não provoca EIO do kernel nem prova durabilidade física.
3. **Concluído — interrupção de syscall via FUSE:** o alvo
   `make test-cp01-fsync-interrupt` passou em Go e Rust. O processo é morto com `SIGKILL`
   enquanto seu `fsync` real está pendente no callback FUSE; depois o callback é
   liberado, a captura recupera o watermark anterior, repete a publicação e
   valida recovery idempotente. Isso prova tratamento de um pedido FUSE em voo,
   não interrupção de um `fsync` já executando no filesystem subjacente, nem
   perda física de energia/cache.
4. **Concluído — queda abrupta QEMU, evidência limitada (commit `07da8d6`):** `make
   test-cp01-qemu-poweroff` passou três execuções, usando `systemd-run --user`
   com filtro seccomp apenas na unidade temporária (`SystemCallFilter=~io_uring_setup`,
   `SystemCallErrorNumber=EPERM`). Isso faz QEMU 11.1.2 usar o fallback epoll sem
   alterar o `RLIMIT_MEMLOCK` 8 MiB nem política global. Em cada execução, Go e
   Rust publicaram e verificaram a captura, QEMU recebeu `SIGKILL`, e novo boot
   recuperou o mesmo TXID/checksum e repetiu recovery idempotente. Guest kernel
   7.3.0-5-cachyos, ext4 sobre virtio-blk, imagem raw com `cache=writeback`; a
   imagem host ficou em `/tmp/opencode` (tmpfs). Isso prova somente restart abrupto
   do VMM/guest com o backing file ainda disponível no host — não perda do host,
   corte físico de energia ou perda de cache de hardware.
5. **Pendente — matriz homologada:** enumerar e executar as combinações
   pretendidas de SQLite/driver, VFS, OS e filesystem; registrar versões e
   resultados por combinação. Não ampliar suporte por extrapolação dos runners
   arm64/macOS atuais.
6. **Pendente por solicitação do operador — CP-01 completo:** deixar em aberto a
   matriz ampliada e revisão; não expor captura WAL nem declarar M2 concluído.
   Avançar somente nos slices independentes do gate. M3 foi concluído; M4 pode
   ser desenvolvido e testado em cópias filesystem isoladas, mas não habilitado
   publicamente como recuperação incremental enquanto CP-01 estiver pendente.

### Fase 3 — S3-compatible e Floci [M3]

**Entregáveis:** interface storage unificada, SDKs S3 idiomáticos, uploads imutáveis, multipart, reconciliação, retry, métricas básicas e testes Floci persistente. Implementação paralela Go/Rust, não compartilhada.

**Gate M3:** Floci persistente + kill/restart + restauração cruzada; operação sem qualquer dependência de S3 real.

**Estado observado em 2026-10-26:** adapters Go/Rust implementados e testados
contra Floci local; estratégia de SDK registrada em ADR 0013. `make test-floci`
passa create-only, upload simples/multipart, retry idempotente, conflito de
bytes/metadados, zero bytes, checksum, HEAD/GET range, paginação, reconciliação
de objetos missing/divergent/unexpected e delete batch. Também publica fixture
LTX com Go, encerra Floci por `SIGKILL`, reinicia com o mesmo volume, baixa e
restaura com Rust; repete na direção Rust→Go. **Gate M3 local Floci verificado.**
Os dois adapters também expõem hooks de métricas de baixa
cardinalidade (operação, duração, bytes e classe de erro), com testes unitários.
Reconciliação read-only (ausentes/divergentes/inesperados com SHA-256 dos bytes)
e testes de retry limitado/exaustão, timeout, cancelamento, classes de erro e
resposta perdida após PUT estão implementados. Limites/capabilities e obrigações
do deployment estão documentados em `spec/STORAGE_CONTRACT.md`; a política do
SDK em ADR 0013. `make test` passou após essas mudanças; a entrega M3 foi
commitada em `1cf06be`: **M3 implementado e gate local verificado.** Evidência
não homologa AWS/R2/outros serviços. CP-01 continua pendente; captura WAL
permanece privada.

### Fase 4 — Recoverability, compaction e retention [M4]

**Entregáveis:** restauração por TXID/timestamp, plano explicável, verificação integral da cadeia, compactação, snapshots periódicos, GC seguro, retenção configurável, runbook DR.

**Gate M4:** restaurar qualquer ponto **prometido pela retenção**; snapshots e LTX ainda necessários nunca são excluídos.

**Estado observado em 2026-10-26 (commit `8ad39a0`):** implementações privadas filesystem em Go e
Rust agora validam inventário/header/checksum, planejam TXID e instante UTC
inclusivo, reconstroem em staging com verificação SQLite, publicam snapshots
compactados em nível superior sem remover fontes e calculam GC somente a partir
de restore points explicitamente fornecidos. GC inicia em dry-run; execução é
uma chamada distinta que refaz o plano e compara SHA-256 do inventário e
identidade do repositório antes de remover objetos. Testes focados cobrem cadeia
real SQLite em Go/Rust, compactação com round-trip, retenção do caminho
prometido e rejeição de plano stale. Essas funções permanecem internas; ainda
não há scheduler periódico, integração de restore/GC com o adapter S3, nem API
pública. **Conformance de snapshot compactado Go→Rust e Rust→Go** passou no
commit `6e15075`, validando TXIDs não iniciais e integridade/conteúdo SQLite.
CP-01 continua bloqueando a habilitação pública de recuperação incremental e
operações que dependam de sua garantia. `docs/recovery-runbook.md` registra o
procedimento M1 disponível e os limites de M4. `make test` passou antes dessa
extensão; os drills CP-01 opt-in permanecem fora desse gate e seu resultado
anterior continua evidência limitada, não aprovação de CP-01.

### Fase 5 — API pública, segurança e observabilidade [M5]

**Entregáveis:** API congelada, `Status`, `Health`, `Flush`, policy de write-gate, integração OTEL, métricas, compatibilidade comprovada de drivers, exemplos com Chi e Axum, guias para uso real e desligamento.

**Gate M5:** exemplos funcionam sem sidecar; armazenamento offline/degradado gera sinal claro e as credenciais não aparecem em logs.

### Fase 6 — Stress, benchmarks, conformance e release [M6]

**Entregáveis:** fuzz/property tests, testes de chaos, benchmarks repetíveis, comparação com Litestream, SBOM, supply-chain checks, release notes, semver, packages Go/Cargo, documentação pública e guia de migração.

**Gate M6:** todos os itens do aceite global aprovados e os binários/pacotes de cada linguagem constroem de maneira autônoma.

### Estimativa de planejamento

- Com 2 engenheiros seniores dedicados (um por linguagem) e revisões cruzadas: aproximadamente **12–20 semanas**, condicionadas especialmente ao protocolo de checkpoint e às falhas descobertas.
- Um único mantenedor, construindo os dois motores em sequência: aproximadamente **6–10 meses** como ordem de grandeza, não compromisso de prazo.
- Não otimizar o cronograma pulando testes de crash, conformance ou corrupção.

## 18. Dependências e paralelização

```text
M0 (spec/fixtures/CI)
 ├── Go: LTX v2 codec ── Go snapshot/restore ── Go WAL capture ── Go S3
 ├── Rust: LTX v2 codec ─ Rust snapshot/restore ─ Rust WAL capture ─ Rust S3
 └── Conformance fixtures + test runner ─────┐
                                             ├── M2 checkpoint gate
Floci S3 bootstrap + chaos transport ────────┤
                                             └── M3 restore cross-language
                                                    ↓
                                     M4 compaction/retention
                                                    ↓
                                  M5 API/observabilidade
                                                    ↓
                                    M6 release e benchmark
```

Permitir desenvolvimento independente de adapters S3 e codecs durante a prova de checkpoint, mas proibir declarar o streaming pronto sem o gate M2. Cada PR deve incluir testes, documentação da alteração e observabilidade aplicável.

## 19. Riscos principais e mitigação

| Risco | Consequência | Mitigação |
|---|---|---|
| WAL reset ou checkpoint durante captura | Perda silenciosa | CP-01, pin e barreira demonstrada, crash/chaos |
| SQLite desatualizado com WAL-reset bug | Corrupção nativa | >= 3.51.3 no runtime de produção |
| Drivers com políticas diferentes de lock | Race/corrupção | Matriz por driver/OS, adapter estreito, fail-fast |
| Formatos LTX v2/v3 indistinguíveis pelo magic | Decode incorreto | Metadata explícita, compatibilidade versionada |
| Manifesto remoto inconsistente | Restore aponta para cadeia errada | Descoberta/validação baseada em LTX imutáveis, reconciliação |
| Queda entre PUT e ACK local | Duplicação/conflito | idempotência por hash e key imutável |
| Storage indisponível e spool cheio | Perda de cobertura / disco cheio | quota, métricas, write gate ou degradação explícita |
| Multi-host com mesmo prefixo | Split-brain | v1 single-writer, sem auto-failover, bloquear conflito conhecido |
| Floci diverge do S3 real | Falsa confiança em portabilidade | capabilities, contratos e limites explícitos; sem simular homologação externa |
| GC remove base ainda necessária | Histórico perdido | mark-and-sweep por grafo, dry-run e restore-test |
| Criptografia sem estratégia de chave | Backup inutilizável | E2E criptografia posterior; provider-managed inicialmente |
| Dependência oculta de benchmark/reference CLI | Falha em produção | binário de referência apenas no CI de conformance |
| Licença de fixtures de terceiros | Risco de distribuição | revisar licenças, preferir golden vectors próprios |

## 20. Critérios de aceite global (Definition of Done)

### Correção

- [ ] Dois motores realmente independentes compilam sem código de replicação do Litestream ou de outro port.
- [ ] SQLite suportado é validado ao iniciar; configurações/combinações inseguras falham explicitamente.
- [ ] Snapshot, WAL incremental, persistência, restauração e compactação têm testes automatizados por linguagem.
- [ ] CP-01 prova segurança da rotação do WAL sob concorrência e crash.
- [ ] Conformance v2 Go ↔ Rust ↔ versão de referência testada passa integralmente.
- [ ] Restauração final executa verificação de integridade e nunca sobrescreve DB ativo implicitamente.
- [ ] Lacunas de TXID, checksum alterado, geração conflitante e arquivo truncado são detectados.
- [ ] Nenhum estado inválido é anunciado como recuperável.

### Operação e integração

- [ ] Aplicações exemplo usam as bibliotecas no mesmo processo, sem daemon/sidecar.
- [ ] Backends filesystem e Floci S3 estão verdes em testes de retry, kill, restart e perda de rede.
- [ ] Status diferencia commit local, spool durável e último remoto confirmado.
- [ ] `Flush(remote=true)` é testado e documentado com timeout e confirmação.
- [ ] Quota de spool, políticas de write-gate e saturação funcionam como especificado.
- [ ] OpenTelemetry/Prometheus têm exemplos integráveis sem HTTP embutido obrigatório.
- [ ] Não há credenciais fixas em exemplos de produção nem secrets em logs.
- [ ] Documentação inclui setup, compatibilidade de drivers, recuperação completa e limites de RPO.

### Qualidade e entrega

- [ ] CI de Go, Rust, conformance, fuzz curto e Floci é reproduzível.
- [ ] Fuzz prolongado e stress de checkpoint/crash registrados no release.
- [ ] Benchmarks apresentam baseline, resultados reais e regressões explicadas.
- [ ] GitHub releases/tag, changelog, SBOM, licença, contribuição e política de security reporting preparados.
- [ ] `go test ./...`, `go vet ./...`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --all-features` e suites integradas passam.
- [ ] Sem features marcadas como prontas sem logs, testes, resultados e critérios de aceite atendidos.

## 21. Documentos ADR a escrever (mínimos)

As ADRs estão registradas em `docs/adr/`: 0001–0013. ADR 0010 define a política
de retenção/compactação para implementação M4; a habilitação pública permanece
bloqueada por CP-01. Partes das ADRs 0011–0012 permanecem propostas, conforme
seus estados.

Cada ADR contém **contexto, decisão, alternativas rejeitadas, consequências, provas/testes e plano de reversão**. Alterações relevantes no contrato exigem atualização simultânea de specs e fixtures.

## 22. Instrução executiva para agente de desenvolvimento

> Desenvolva o projeto por jornadas verticais, seguindo o presente plano. O
> primeiro vertical M1 está implementado em Go e Rust; M0 inclui parsers WAL
> isolados e vetores compartilhados. O protótipo de captura permanece interno:
> não o exponha pela API pública nem considere o motor incremental seguro até
> passar CP-01 com sincronização/rotação WAL e testes concorrentes/crash.
> Continue os testes de conformance conforme as dependências, registrando
> decisões em ADRs. Execute `make test` para o gate completo. Nenhum serviço
> externo real é necessário. Sempre entregue código, testes, docs, evidências e
> pendências remanescentes por milestone; jamais declare algo implementado
> apenas porque consta do plano.

## 23. Referências técnicas e normativas

- SQLite — WAL, checkpoint e bug de reset: https://www.sqlite.org/wal.html
- SQLite — formato dos arquivos DB/WAL e checksums: https://www.sqlite.org/fileformat2.html
- SQLite — Online Backup API: https://www.sqlite.org/c3ref/backup_finish.html
- SQLite — WAL hook: https://www.sqlite.org/c3ref/wal_hook.html
- Litestream — funcionamento, TXID, compactação e recuperação: https://litestream.io/how-it-works/
- Litestream — especificação e layout LTX documentados: https://github.com/benbjohnson/litestream/blob/main/docs/LTX_FORMAT.md
- Superfly LTX — descrição v3 e ambiguidade de versão: https://github.com/superfly/ltx/blob/main/README.md
- Go `modernc.org/sqlite`: https://pkg.go.dev/modernc.org/sqlite
- Rust `rusqlite`: https://github.com/rusqlite/rusqlite
- Floci — execução, variáveis e persistência: https://github.com/floci-io/floci
- Floci — releases: https://github.com/floci-io/floci/releases
- AWS — escrita condicional S3 (sem exigir ambiente AWS nos testes): https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html

---

**Estado do plano em 2026-10-26:** M0 parcial, M1 implementado/verificado, M2
bloqueado por CP-01, M3 implementado e verificado no gate local Floci; M4–M6
incompletos. O sub-drill QEMU foi repetido três vezes, mas sua evidência
limitada não aprova CP-01. Por solicitação do operador, o gate completo fica
pendente enquanto avançam slices independentes; a captura WAL permanece privada.
M3 inclui reconciliação read-only, falhas/retry/cancelamento testados e métricas
básicas instrumentadas. `make test` passou integralmente; drills CP-01 não foram
executados nesta validação. A compatibilidade comprovada continua sendo LTX v2 contra
`superfly/ltx v0.4.0`, não qualquer release genérica do Litestream.
