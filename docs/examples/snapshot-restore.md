# Snapshot e restore offline

Os exemplos mínimos usam um banco SQLite já existente em modo WAL e criam um
snapshot LTX v2 em um repositório filesystem. O destino do restore deve ser um
caminho novo; os exemplos não sobrescrevem arquivos existentes.

```sh
cd go
go run ./examples/snapshot-restore /path/to/app.sqlite /path/to/replica /path/to/restored.sqlite
```

```sh
cargo run --manifest-path rust/Cargo.toml -p sqlite-replication-core \
  --example snapshot-restore -- /path/to/app.sqlite /path/to/replica /path/to/restored.sqlite
```

O exemplo não inicia replicação incremental, não configura storage remoto e não
substitui a aplicação na escolha do lifecycle/conexões. WAL incremental segue
bloqueado pelo CP-01.
