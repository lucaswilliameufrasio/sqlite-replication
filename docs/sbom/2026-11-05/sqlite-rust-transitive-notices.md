# Grafo e notices do adapter SQLite Rust — triagem parcial

Em 2026-11-05, o grafo normal e de build de `sqlite-replication-sqlite` foi
consultado com Cargo 1.99.1, lockfile local e target Linux x86-64:

```sh
cargo tree --offline --locked -p sqlite-replication-sqlite \
  -e normal,build --target x86_64-unknown-linux-gnu
```

O resultado é uma resolução de dependências, não um inventário comprovado de
bytes em todo binário ou imagem; as features podem ser unificadas por outros
consumidores. Não é parecer jurídico nem aprovação de distribuição.

## Seleção e textos nos pacotes locais

Na revisão posterior do candidato `341d682`, os checksums das archives `.crate`
do cache coincidiram com o `rust/Cargo.lock` incluído: `rusqlite 0.41.1`,
`da9bba6f3506135e120a6586df05be18b48d18896600646a4bec8f993d91a594`,
e `libsqlite3-sys 0.39.1`,
`14a5f01b87b92705d07b433c06f96cbc764637d81a0bffb5734ce17cf0f9a2b3`.
Busca em todos os 72 e 22 membros, respectivamente, por basename iniciado por
`LICENSE`, `NOTICE` ou `COPYING`, sem distinção de maiúsculas, encontrou apenas
o `LICENSE` de cada pacote, com o hash MIT da tabela abaixo. O `sqlite3.c`
na archive de `libsqlite3-sys` também corresponde ao hash abaixo. A busca por
nomes não exclui avisos embutidos; o MIT do wrapper não resolve os termos da
amalgamação. Essas dependências não são vendorizadas na archive-fonte do
candidato e sua presença num binário final não é estabelecida por esta revisão.

| Componente | Papel no grafo | Licença no manifesto / evidência local |
|---|---|---|
| `rusqlite 0.41.1` | wrapper normal | `MIT`; `LICENSE`, SHA-256 `c10c1f27337546471e5f7e4e97fdd398b35b9d4e126115dcd22de8d8e65abf6f` |
| `libsqlite3-sys 0.39.1` | bindings e build nativo | `MIT` para o wrapper; `LICENSE` com mesmo hash acima; não descreve sozinho a amalgamação SQLite |
| `bitflags 2.13.2`, `smallvec 1.16.4` | normais | `MIT OR Apache-2.0`; cada cache possui `LICENSE-MIT` e `LICENSE-APACHE` |
| `fallible-iterator 0.3.0`, `fallible-streaming-iterator 0.1.9` | normais | manifesto usa literalmente `MIT/Apache-2.0`; cada cache possui textos MIT e Apache separados; revisar a interpretação da expressão e escolha aplicável |
| `cc 1.11.0`, `find-msvc-tools 0.1.15`, `shlex 2.0.1`, `pkg-config 0.3.34` | build e transitivos de build | `MIT OR Apache-2.0`; caches possuem ambos os textos |
| `vcpkg 0.2.15` | build | manifesto usa literalmente `MIT/Apache-2.0`; cache possui ambos os textos |

Foram inspecionados nomes de arquivos `LICENSE*`, `NOTICE*` e `COPYING*` na
raiz desses 11 pacotes; não foram encontrados `NOTICE*` ou `COPYING*` ali. Isso
não verifica textos aninhados, avisos nos fontes, a totalidade de terceiros ou
obrigações de ferramentas incluídas em uma imagem. O `LICENSE` MIT de rusqlite
e libsqlite3-sys identifica copyright 2014 dos desenvolvedores rusqlite e exige
incluir copyright e permissão em cópias ou partes substanciais. Textos completos
e disclaimers precisam ser revisados por artefato, sem reduzir a atribuição ao
nome de uma licença.

## SQLite incluído

O grafo de features do checkout selecionou `rusqlite/backup` e `bundled`, este
último ativando `libsqlite3-sys/bundled` e a ferramenta `cc`. A cópia local de
`libsqlite3-sys` contém `sqlite3/sqlite3.c`, SHA-256
`0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52`, que
define SQLite **3.54.1** e contém disclaimers de copyright dos autores em
diversos trechos. Isso não autoriza tratar automaticamente todo o pacote C,
extensões ou outros arquivos como cobertos pela licença MIT do wrapper. Revisar
a procedência da amalgamação, documentação upstream, arquivos embutidos,
condições locais e aplicabilidade da declaração de domínio público antes de
compor os notices finais. O resultado de `nm` para um executável local específico
foi registrado separadamente em `native-notices-review.md`; não extrapolá-lo
para todas as opções de build.

O grafo de build lista `pkg-config` e `vcpkg` por features selecionadas, mas a
listagem, por si só, não prova que cada ferramenta executou ou foi distribuída.
Por outro lado, código de build pode ser relevante ao distribuir fontes; revisar
o escopo real. Dependências `dev` como LTX, WAL e `tempfile` não estão nesta
tabela: os testes do pacote extraído já demonstraram problemas de resolução
documentados em `distribution-candidate-inventory.md`.

Faltam definir os artefatos de distribuição, avaliar outras árvores transitivas
(em particular S3/AWS-LC), notices aninhados, fixtures e direitos, e aprovação
do mantenedor. Não publicar, criar licença final ou alegar conformidade com base
apenas nesta triagem.
