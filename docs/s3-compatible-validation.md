# Validação de endpoints S3-compatible

Os adapters Go e Rust implementam o contrato S3 em `spec/STORAGE_CONTRACT.md`.
Compatibilidade com um provedor específico depende de evidência para a versão,
configuração e operações testadas; Floci ou RustFS não homologam outro serviço.

## Testes locais

`make test-floci` usa Floci local. `make test-rustfs` usa um projeto Docker
rootless isolado, bucket e volume descartáveis, endpoint em loopback e o
checkout Go indicado por `GO_REPO`. O script usa credenciais sintéticas somente
para esse container: nunca use essa configuração para um serviço real.

Em 2026-11-09, duas execuções locais passaram com RustFS 1.0.4
(`sha256:ed9210d6411d77ffceb70d94b964a3cb7d225983368bc25bebe01e995293dcc2`),
usando o Go `27d6d32297f335b1bd33549eb06fc495d58ba01f` e o checkout Rust
baseado em `5d0799c419f0325da8b7d999f7d67dc0bc1255ec`. A segunda execução
confirmou explicitamente os testes Go de restore cruzado, além dos contratos
Go/Rust, recuperação, persistência após SIGKILL e barreira/ACK Rust. Os nomes
dos testes e algumas mensagens ainda dizem Floci porque a suíte existente é
reutilizada com um endpoint RustFS local. Isso não certifica outras versões ou
configurações de RustFS nem os demais provedores.

## Roteiro para usuários de serviços reais

R2, AWS S3 e Backblaze B2 S3-compatible ainda não foram validados por este
projeto. O proprietário deve usar uma conta de teste, bucket e prefixo
exclusivos, dados sintéticos e permissões mínimas. Não use o bucket de produção
nem compartilhe credenciais, URLs assinadas, conteúdo de objetos ou logs com
segredos. Mantenha os backups filesystem existentes até testar o restore.

Para cada linguagem, informe revisão da biblioteca, provedor, versão ou data,
região, estilo de endereçamento e resultado de cada cenário:

1. Configurar o cliente SDK com endpoint, região e credenciais apropriados;
   confirmar conexão e erros de autenticação sem registrar segredos.
2. Enviar objeto pequeno e vazio; ler, verificar tamanho e SHA-256 e testar
   leitura parcial e listagem paginada.
3. Repetir um upload idêntico e tentar conteúdo diferente na mesma chave;
   verificar que o primeiro é idempotente e o segundo não sobrescreve o objeto.
4. Repetir com objeto acima de 8 MiB para exercitar multipart e sua conclusão
   condicional. Se as precondições não forem respeitadas, interromper: não
   degradar para overwrite.
5. Exercitar timeout, retry e resposta perdida quando viável; confirmar que não
   se anuncia ACK remoto sem publicação confirmada.
6. Criar um banco sintético, capturar e sincronizar, restaurar em caminho novo,
   executar verificação de integridade SQLite e conferir os dados. Se possível,
   testar publicação Go → restore Rust e vice-versa.
7. Remover somente os objetos de teste e conferir a política de expiração de
   multipart incompletos. Não execute GC contra dados de produção como teste.

Esses passos são um roteiro de validação externa, não uma alegação de que os
serviços foram testados nem autorização para acessar contas de usuários.
