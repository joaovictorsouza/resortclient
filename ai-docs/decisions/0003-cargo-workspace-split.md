# ADR 0003: Separação em Cargo Workspace (protocol, nakama_client, game)

## Status
Aceita

## Contexto
Projetos Bevy grandes sofrem com tempos de compilação lentos caso todo o código resida em um único pacote monolítico. Além disso, os tipos de protocolo de rede não devem depender de Bevy.

## Decisão
Estruturar o cliente em 3 crates em um Cargo Workspace:
1. `protocol`: tipos puros com `serde`.
2. `nakama_client`: camada assíncrona de rede Tokio.
3. `game`: motor Bevy, renderização, física e launcher desktop/mobile.

## Consequências
- Compilações incrementais significativamente mais rápidas.
- Testes unitários de protocolo e rede executam em milissegundos sem compilar o Bevy.
