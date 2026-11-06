# Cadeia transitiva LTX Rust — revisão parcial de notices

Investigação em 2026-11-05 do crate local `sqlite-replication-ltx` para Linux
x86-64, com Cargo 1.99.1 e lockfile presente. Não é parecer jurídico, prova de
licenciamento dos artefatos finais ou autorização de distribuição. A revisão
considera o grafo normal selecionado, não todos os recursos que outro consumidor
pode unificar, dependências de desenvolvimento dos terceiros ou código
efetivamente vinculado em um executável específico.

## Grafo e textos locais

`cargo tree --offline --locked -p sqlite-replication-ltx -e normal,features
--target x86_64-unknown-linux-gnu` mostrou `lz4_flex 0.14.0`, com features
padrão incluindo `frame`, e sua dependência `twox-hash 2.1.5` com `xxhash32`.
O manifesto de `lz4_flex` desabilita as features padrão de `twox-hash` nessa
dependência; o grafo não selecionou `random` ou `serde`. Ambos os manifests
declaram MIT. Isso descreve esta resolução, não a união de features de outro
workspace nem garante que cada função apareça em binário final.

| Crate / fonte local | Texto verificado | SHA-256 |
|---|---|---|
| `lz4_flex 0.14.0`, `LICENSE` | MIT; copyright 2020 Pascal Seitz | `0982c33390159842ecce8e9d6ce2e5e39961fe0e0ffbb2eec39c9ba46db6db10` |
| `twox-hash 2.1.5`, `LICENSE.txt` | MIT; copyright 2015 Jake Goulding | `f59bcb1c02654665f78a5f290bae76b6358137675f6459f51e6d474a4623d8ea` |

Os textos MIT examinados requerem inclusão do aviso de copyright e permissão
nas cópias ou porções substanciais. Devem ser preservados os textos completos
aplicáveis, incluindo disclaimers, não apenas a palavra `MIT`. Uma busca por
nomes `LICENSE`, `NOTICE` e `COPYING` nas árvores locais dessas crates encontrou
somente os dois arquivos acima; isso não elimina avisos em comentários ou a
necessidade de revisar o conteúdo efetivamente distribuído.

## Trecho da especificação LZ4

`lz4_flex 0.14.0/src/frame/header.rs:93-126` atribui ao documento upstream
`lz4_Frame_format.md` uma tabela de formato reproduzida em comentário. O
documento consultado em 2026-11-05 na árvore `dev`, commit
[`cb3b375558c3d2b62708d1d7ba6a08d5c25fe72a`](https://github.com/lz4/lz4/blob/cb3b375558c3d2b62708d1d7ba6a08d5c25fe72a/doc/lz4_Frame_format.md),
apresenta notice de copyright de Yann Collet e permissão para copiar e
distribuir o documento condicionada à preservação do copyright e notice e à
marcação de mudanças ou exclusões substantivas. **O commit exato da versão de
origem da tabela na crate ainda não foi estabelecido**; a consulta ao `dev` não
prova qual texto ou condição se aplicava no momento da cópia. Revisar origem,
termos e eventual atribuição aplicável se fontes ou documentação da dependência
forem distribuídas; não tratar a expressão MIT do manifesto como prova de que
todos os trechos de terceiros foram analisados. A investigação histórica abaixo
reduz essa incerteza sem eliminá-la.

### Comparação histórica

O `.cargo_vcs_info.json` da crate `lz4_flex 0.14.0` aponta para
`1bffdcbbf906a234b913937cb2f57c0245915038`. O histórico upstream de
`src/frame/header.rs` mostra que o commit
[`dddaac45704d9f6d839b8553e2646e353c0b3a91`](https://github.com/pseitz/lz4_flex/commit/dddaac45704d9f6d839b8553e2646e353c0b3a91),
de 2021-03-29, adicionou a atribuição ao caminho `dev` e as tabelas em comentário.
O link aponta para branch móvel, não para um commit da especificação.

A última alteração retornada pela API GitHub para o caminho
`doc/lz4_Frame_format.md` até aquela data foi o commit LZ4
[`87a80acbe7872b9da7d56f7005ffd1b715e87c93`](https://github.com/lz4/lz4/blob/87a80acbe7872b9da7d56f7005ffd1b715e87c93/doc/lz4_Frame_format.md),
de 2020-11-25. O documento identifica a versão **1.6.2 (12/08/2020)** e contém
notice `Copyright (c) 2013-2020 Yann Collet`, com permissão de cópia e
distribuição condicionada à preservação do copyright e notice e à marcação de
alterações ou exclusões substantivas. O arquivo obtido desse commit teve
SHA-256 `5887a23530bd7bd42d86a2d64b2b72bcad68618ae1febe4f5fb65e80082dc735`.

As **15 linhas de tabela** presentes no comentário da crate local correspondem
exatamente a linhas do documento histórico; cabeçalhos e texto intermediário são
um recorte, não uma reprodução integral. Isso apoia uma origem compatível com
a versão 1.6.2, mas não prova o commit efetivamente consultado pelo autor,
o estado da branch `dev` naquele instante, nem resolve a interpretação jurídica
do recorte. Antes de distribuir a fonte correspondente, revisar o notice
histórico completo e como preservar a atribuição e identificar as omissões ou
alterações pertinentes. Não incluir automaticamente este resumo como notice
final nem afirmar que a questão legal foi concluída.

O pacote LTX também contém referências a fixtures externas que não são incluídas
no `.crate`; testes e benchmark do pacote extraído falharam por ausência desses
arquivos. Direitos das fixtures e decisões de empacotamento continuam pendentes
em `fixture-provenance-review.md` e `distribution-candidate-inventory.md`.
O gate integral e a regressão de admissão são independentes desta revisão.
