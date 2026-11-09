# ADR 0014: caminho do módulo Go e licença dos pacotes

- **Status:** aceita para caminho do módulo e escolha da licença; revisão de atribuições e publicação pendentes
- **Data:** 2026-10-27

## Contexto

O `go.mod` usava `sqlite-replication`, identificador provisório que não fornece
um caminho de import público canônico. Os pacotes Rust já declaram Apache-2.0;
a publicação ainda depende de revisar notices de dependências e direitos das
fixtures.

## Decisão

- O caminho canônico Go será
  `github.com/lucaswilliameufrasio/sqlite-replication`, correspondente ao
  `origin` GitHub sanitizado e à opção GitHub owner/repository escolhida pelo
  mantenedor.
- Apache-2.0 será a licença pretendida para Go e Rust, sujeita à revisão de
  notices/atribuições de dependências e à confirmação dos direitos das fixtures.
- A pedido do mantenedor, o aviso de copyright usará o nome público do perfil
  GitHub, **Lucas Eufrasio** (`https://github.com/lucaswilliameufrasio`), com
  ano inicial 2026. O perfil público não foi tratado como verificação de nome
  civil/legal.
- Ainda não criar o arquivo `LICENSE` nem publicar pacote até a revisão de
  notices/dependências e confirmação dos direitos das fixtures.
- Esta decisão não autoriza tag, release, publicação ou push; tampouco altera o
  gate CP-01 ou torna captura WAL pública.

## Alternativas consideradas

- **Domínio vanity:** desacoplado do host, mas exigiria domínio controlado e
  manutenção contínua dos metadados HTTPS; não foi escolhido.
- **Manter `sqlite-replication`:** adequado apenas como nome interno, não como
  caminho Go público estável; substituído antes da primeira publicação.
- **Outra licença:** não escolhida; Apache-2.0 mantém alinhamento com o metadata
  Cargo atual, sujeito às verificações jurídicas/de atribuição pendentes.

## Consequências

O `go.mod` e os imports internos devem usar o caminho GitHub completo. Como ainda
não houve publicação, a alteração ocorre antes do primeiro contrato de import
externo. Se o repositório mudar de owner/path antes do primeiro release, revisar
esta ADR e o módulo antes de publicar.

## Evidência pendente

- Revisar licenças/notices de dependências e permissões das fixtures antes de
  criar o pacote legal final ou publicar. Os geradores de fixtures e a
  proveniência LTX/WAL estão descritos em `conformance/fixtures/README.md` e
  `conformance/fixtures/wal/README.md`; ainda falta a revisão de atribuições.
