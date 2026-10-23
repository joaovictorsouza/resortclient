---
name: protocol-change-client
description: >-
  Acompanha no CLIENTE (Rust) uma mudança do contrato de rede feita no servidor: sincroniza
  `contract/`, atualiza opcodes/structs e roda os testes de roundtrip com as fixtures.
  Use quando o servidor publicar uma nova versão do contrato.
---

# Mudança de Protocolo de Rede: lado do cliente (consumidor)

O servidor é o dono do contrato (skill `protocol-change`). O cliente guarda uma cópia em `contract/`
(`CONTRACT_VERSION`, `fixtures/`, `furniture-catalog.json`). **Nunca edite `contract/` à mão.**

## Procedimento Obrigatório

1. **Ler a mudança**: PR/issue do servidor e [resort-game-server/contract/protocol.md](https://github.com/joaovictorsouza/resort-game-server/blob/develop/contract/protocol.md) na versão alvo.
2. **Sincronizar o contrato**:
   ```bash
   ./scripts/sync-contract.sh      # copia resort-game-server/contract -> contract e o catálogo de móveis para assets/
   ```
   (Depois da separação dos repositórios, o script baixa a versão publicada pelo servidor.)
3. **Implementar no cliente (Rust)**:
   - Opcode em `crates/protocol/src/opcodes.rs`; structs com `#[derive(Serialize, Deserialize)]` em `messages.rs`.
   - Atualize `decode_server_message` e `encode_client_message` em `lib.rs`.
   - Testes de roundtrip usando as fixtures de `contract/fixtures/` (`include_str!`).
   - Execute `cargo test -p protocol`.
4. **Compatibilidade**: o cliente deve tolerar campos ausentes (`#[serde(default)]`) enquanto houver servidor antigo em uso.
5. **Validar**: `./scripts/check.sh`. O check `contract in sync` (`scripts/check-contract.sh`) falha se `contract/` divergir do servidor.
