# Migração da toolchain Rust

O workspace passou a declarar Rust **1.94.1** como versão mínima, em vez de
1.85, devido às dependências atualizadas do AWS SDK. A linha de base usada no
desenvolvimento e no CI é **Rust/Cargo 1.99.1**, fixada em `.mise.toml`. A MSRV
declarada ainda não foi validada por uma execução do gate com Rust 1.94.1.

Antes de atualizar a dependência desta biblioteca, instale uma toolchain
compatível, atualize o lockfile do projeto consumidor e execute os testes da
aplicação. Compiladores anteriores a 1.94.1 deixam de ser suportados. Projetos
que usam `mise` podem usar os pins do repositório como referência.

O adapter S3 usa o cliente HTTPS padrão moderno do AWS SDK, Tokio, HTTP 1.x e
SigV4a. A feature `rustls` do SDK, que ativa também o conector legado, não é
selecionada diretamente por este workspace. Features são unificadas pelo Cargo:
outra dependência ou o próprio consumidor ainda podem reativá-la. Audite a
árvore e o lockfile finais da aplicação; um resultado limpo neste repositório
não garante que toda árvore consumidora esteja livre de advisories. A presença
de certificados raiz válidos no ambiente continua necessária para HTTPS.

O teste Floci é local e não homologa AWS S3, Cloudflare R2 ou outros provedores.
As garantias de captura WAL permanecem restritas às combinações Linux x86-64,
Go/Rust/SQLite verificadas pelos drills CP-01; esta migração não amplia esse
escopo nem comprova proteção contra perda física de energia ou cache.
