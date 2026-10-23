# Definition of Done (Client)

Uma funcionalidade no cliente só é aceita quando:

1. [ ] A lógica de suporte possui testes unitários automatizados.
2. [ ] `./scripts/check.sh` roda com zero erros de compilação, formatação e lints.
3. [ ] A integração com os eventos de rede foi testada contra o contrato oficial em `resort-game-server/contract/protocol.md` e suas fixtures (`contract/`, sincronizado por `scripts/sync-contract.sh`; `scripts/check-contract.sh` falha se divergir).
4. [ ] O código não quebra compatibilidade com builds mobile (`crates/game/src/lib.rs`).
5. [ ] A funcionalidade foi verificada no launcher desktop `cargo run -p game`.
