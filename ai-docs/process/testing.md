# Estratégia de Testes no Cliente

## Metas de Cobertura e Confiabilidade

- **Crate `protocol`**: **≥ 80%** de cobertura, cobrindo serialização de todos os opcodes e roundtrips com serde.
- **Crate `nakama_client`**: Testes de parsing de envelopes WebSocket e despacho de comandos.
- **Crate `game`**: Testes unitários para lógica pura (ex: `tile_to_world`, `world_to_tile`, interpolação). Testes de sistemas ECS podem usar `App::new().add_plugins(MinimalPlugins)`.

## Executando a Suíte de Testes
```bash
cd client
cargo test --workspace --verbose
```
