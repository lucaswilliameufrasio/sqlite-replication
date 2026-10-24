# Compatibilidade LTX v3 — adiada

O perfil implementado é somente LTX v2 (`superfly/ltx v0.4.0` como oráculo
test-only). LTX v2 e v3 usam o mesmo magic `LTX1`; portanto magic não identifica
versão.

LTX v3 não é aceito no perfil atual: frames, campos de tamanho/compressão e
validação diferem. O decoder v2 falha fechado ao encontrar layout incompatível;
nenhuma heurística tenta reinterpretar bytes como v2. Metadata externo ou
configuração explícita deve escolher versão antes de abrir o codec.

Adicionar v3 exige uma matriz separada de versão de referência, spec wire,
fixtures independentes, leitores/writers por linguagem, regras de layout/chain,
testes de rejeição cruzada e estratégia de migração que reconstrua e valide o
banco. Não misturar v2 e v3 sob a mesma cadeia/prefixo sem migração explícita.
