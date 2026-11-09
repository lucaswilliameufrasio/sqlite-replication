# Matriz de notices — candidato local Rust, não aprovado

## Composição por artefato do candidato `413e08d`

Na archive local `candidate-413e08d-review-a.tar.gz`, SHA-256
`ebf437a4983641b104494ea546956ea0e33a640bd225091370f9158b3c8f6657`,
a inspeção dos nomes confirmou 44 fixtures LTX e três WAL, nove manifests
Cargo e dois lockfiles, incluindo os do exemplo separado, e 16 documentos
CycloneDX. Três destes documentos têm `evidence-quarantined` no nome: os dois
SBOMs Go gerais e o SBOM Go de testes de telemetria; foram preservados.

Os dados de fixtures são incluídos efetivamente; sua reprodução documentada
não confirma direitos. Os lockfiles e SBOMs descrevem dependências, mas não
equivalem a fontes vendorizados ou a binários distribuídos. Em particular, os
notices e objetos nativos de SQLite e AWS-LC continuam dependentes do artefato,
features e target finais. O exemplo de telemetria continua sem declaração de
licença própria no manifesto. Esta inspeção é posterior à archive e não integra
seu conteúdo; não aprova direitos, notices ou distribuição.

## Manifests e avisos textuais do candidato poison

Inspeção direta de `candidate-20261106-poison-a.tar.gz`, SHA-256
`0354c5ad73181bb207cec3f8f99ab4d837070f9966b204aa6f161c534e036ab6`:
os sete manifests de `rust/crates/` usam `license.workspace = true`; o
`rust/Cargo.toml` dessa archive declara `workspace.package.license = "Apache-2.0"`.
Já `rust/examples/coordinator-telemetry/Cargo.toml` não declara `license` nem
`license-file`: é workspace separado, portanto a declaração do workspace principal
não deve ser atribuída automaticamente ao exemplo. Confirmar direitos e licença
do exemplo é pendência específica; não alterar o manifesto para presumir aprovação.

Uma busca limitada aos 30 arquivos `.rs` incluídos, insensível a maiúsculas, por
`copyright`, `spdx`, `license`, `licensed` ou `attribution`, não encontrou
correspondências. Isso não identifica trechos copiados sem esses termos, não
examina binários nem resolve proveniência ou obrigações. Este registro é
posterior à archive e não integra o candidato inspecionado; nenhuma distribuição
está autorizada.

## Candidato com teste de sobreposição de captura

`candidate-20261106-overlap-a.tar.gz`, SHA-256
`c9b3e3752bd770eb65920f92f16d73fe89f863a4ef1ae031f3cfc9d58287ad5b`,
foi gerado duas vezes com hashes iguais. A extração isolada conferiu os 119
arquivos do manifesto e os dois controles. A seção de escopo por artefato
abaixo já constava no checkout antes da geração; esta nota é posterior e não
integra a archive. Direitos, textos completos aplicáveis e aprovação para
distribuição continuam pendentes; os testes da extração são verificação
separada da revisão de notices.

## Inspeção do candidato com caracterização de mutex

A archive local `candidate-20261106-mutex-a.tar.gz`, SHA-256
`9dc0583490c863c8b817b414302e205248bf9ef3f850e0c88557ee62badedfd7`,
contém 42 arquivos Rust, 50 fixtures, 27 arquivos de evidência SBOM e dois
arquivos de controle. A extração verificou os 119 hashes do manifesto. Não há
arquivo com basename exato `LICENSE`, `NOTICE` ou `COPYING`; isso não elimina
a possibilidade de avisos embutidos ou com outros nomes. Os textos integrais,
direitos e autorização continuam pendentes. Esta nota é posterior à archive.

### Escopo por artefato inspecionado

Confrontando o `SHA256SUMS` e a extração desse candidato com as revisões de
proveniência e de AWS-LC, distinguem-se:

| Item | Presença observada neste candidato | Limite da evidência |
|---|---|---|
| Fixtures | 44 arquivos `.ltx` e três `.wal`, além de três arquivos de documentação e hashes. | São dados efetivamente incluídos; reprodução dos LTX não confirma direitos. Os geradores e o oráculo não estão na archive. |
| S3 / AWS-LC | `rust/crates/storage-s3/`, `rust/Cargo.lock` e a revisão `s3-aws-lc-transitive-notices.md` estão presentes. | O grafo documentado identifica `aws-lc-sys`, mas o candidato não inclui sua árvore vendorizada nem binário compilado. Isso não determina obrigações de um binário futuro. |
| Telemetria Rust | `rust/examples/coordinator-telemetry/` inclui fontes, testes e seu próprio `Cargo.lock`; há SBOM e revisão separados em `docs/sbom/2026-11-05/telemetry-examples/`. | A presença do exemplo não significa que suas dependências ou binários estejam incluídos; a revisão específica permanece necessária. |
| SBOMs Go e quarentena | Documentos CycloneDX Go, inclusive cópias com `evidence-quarantined` no nome, estão efetivamente na archive. | São documentos de evidência, não os fontes Go ou uma prova da composição de um binário. Não remover nem tratar a quarentena como licença resolvida. |

Esta inspeção é de composição, não uma conclusão sobre quais notices são
juridicamente necessários. Fontes próprios podem conter trechos e avisos
embutidos; a ausência de uma árvore vendorizada não encerra essa revisão.
Nenhum binário de distribuição foi selecionado ou auditado aqui. Esta seção
foi adicionada após a geração e não integra a archive identificada acima.

## Candidato de reconciliação sincronizada

`candidate-20261106-sync-a.tar.gz`, SHA-256
`2bf79e1414a5c216982bb7131142a1b451784d496a2d0728990d122d71790978`,
contém 42 arquivos Rust, 50 fixtures, 27 arquivos de evidência SBOM e dois
arquivos de controle. A extração verificou 119 hashes e não encontrou nomes
exatos `LICENSE`, `NOTICE` ou `COPYING`; isso não exclui avisos com outros
nomes ou embutidos. A revisão por componente abaixo continua preliminar:
faltam textos integrais aplicáveis, confirmação de direitos e autorização.
Esta anotação é posterior à geração e não está nessa archive.

## Nova inspeção da composição — correção de invalidação

A archive local `candidate-20261106-invalidate-a.tar.gz`, SHA-256
`e0cb8b03020c6224cc883acec13db22ad4613b5ddb5fa06794e0feea87618737`,
contém 121 membros: 42 sob `rust/`, 50 fixtures, 27 em `docs/sbom/2026-11-05/`
e dois arquivos de controle. A extração verificou os 119 hashes do manifesto;
há 16 arquivos CycloneDX e nenhum membro com basename exatamente `LICENSE`,
`NOTICE` ou `COPYING`. Busca por basename não exclui avisos em outros nomes ou
embutidos nos fontes. A matriz abaixo se refere à archive anterior, mas as
mesmas pendências de textos completos, direitos de fontes e fixtures e revisão
por artefato permanecem abertas. Esta atualização é posterior à nova archive.
Não há aprovação de distribuição ou conclusão jurídica nessa inspeção.

Esta é uma triagem técnica do arquivo local `candidate-20261106-a.tar.gz`,
SHA-256 `6ed8433053734727a438b3027e18809b99a16d6bda3a9dc20cdd71fcbb257f31`.
Não é um pacote final de notices, parecer jurídico ou autorização para
distribuir. O conteúdo corresponde ao checkout local na geração e não inclui
alterações feitas posteriormente neste documento.

## Conteúdo observado na archive

Inspeção de nomes com Python `tarfile`: 119 membros regulares: 41 sob `rust/`,
50 sob `conformance/fixtures/`, 26 sob `docs/sbom/2026-11-05/` e dois arquivos
de inventário/aviso na raiz. Há 16 documentos CycloneDX, incluindo originais e
cópias de quarentena. Nenhum membro tem basename exatamente `LICENSE`,
`NOTICE` ou `COPYING`. Essa busca por nome não identifica todos os possíveis
avisos embutidos nos fontes nem os termos aplicáveis a cada arquivo.

| Conteúdo | Evidência disponível | O que falta para distribuição |
|---|---|---|
| Fontes e manifests Rust, inclusive exemplo de telemetria | Manifests indicam a licença pretendida; o exemplo é workspace separado. | Confirmar direitos das contribuições e a licença final do projeto; a archive não contém texto `LICENSE` aprovado. |
| Fixtures LTX e WAL | `fixture-provenance-review.md` descreve geradores e reprodução; os dados estão na archive. | Confirmar direitos de geradores, contribuições e dados efetivamente incluídos; reprodução por hash não concede licença. |
| Dependências de compilação e execução | `Cargo.lock` e SBOMs registram resolução; os fontes de dependências não são vendorizados. | Determinar o tratamento aplicável ao pacote fonte e a cada binário/feature; não presumir que dependências do lockfile estejam incluídas na archive. |
| SQLite e código nativo de S3 | Revisões em `sqlite-rust-transitive-notices.md`, `native-notices-review.md` e `s3-aws-lc-transitive-notices.md`. | Revisar textos completos, fontes nativas, objetos realmente vinculados e obrigações de cada binário. |
| LTX/LZ4 Rust | `ltx-transitive-notices.md` registra versões e textos examinados no cache. | Identificar os textos e atribuições exigidos pelo artefato escolhido, sem generalizar a todas as features/targets. |
| Exemplo de telemetria | `telemetry-examples/upstream-notices-review.md` e SBOMs separados. | Definir se o exemplo e suas dependências serão distribuídos e revisar seus avisos nessa configuração. |

Os documentos de revisão listados são evidência **resumida**, não cópias
completas de todas as licenças e disclaimers. Os SBOMs não são uma lista final
de notices, não demonstram quais símbolos foram vinculados e incluem escopos
diferentes do candidato; preservar a quarentena de classificações controversas.
Antes de qualquer distribuição, revisar a composição efetiva, obter e conferir
os textos integrais aplicáveis, confirmar direitos das contribuições e fixtures,
decidir a licença final do projeto e obter autorização específica. Não há
garantia de cobertura por target além das combinações Linux x86-64 testadas.
