# Fluxo de Desenvolvimento no Cliente (Bevy / Rust)

## Passo a Passo para Desenvolver uma Feature

1. **Leitura da Spec**:
   - Consulte ou redija a especificação em [ai-docs/specs/](../specs/) usando a skill `write-spec`.
2. **Implementação de Lógica Pura Primeiro**:
   - Crie cálculos de geometria, interpolação, parsing e lógica em funções puras independentes de `World` ou `ResMut`.
   - Adicione testes unitários no módulo `#[cfg(test)]`.
3. **Criação do Plugin Bevy**:
   - Agrupe componentes, recursos, eventos e sistemas sob um `Plugin` dedicado (ex: `InventoryPlugin`, `FurniturePlugin`).
   - Conecte o plugin na lista de plugins do `GamePlugin` em `crates/game/src/lib.rs`.
4. **Verificação de Qualidade**:
   - Execute `./scripts/check.sh` na raiz do repositório.
   - Resolva todos os avisos do `cargo fmt` e `cargo clippy`.
5. **Teste Visual Desktop**:
   - Execute `cargo run -p game` para validar o comportamento e interação.
