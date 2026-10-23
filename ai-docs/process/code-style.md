# Guia de Estilo de Código (Rust / Bevy)

- **Organização Modular**: Cada subsistema deve ter seu próprio módulo e `Plugin`.
- **Idiomatic Rust**: Nunca use `unwrap()` ou `expect()` em fluxos de runtime que possam receber dados inválidos de rede; propague erros com `Result` ou use `warn!`.
- **Tratamento de Estado**: Utilize `AppState` do Bevy e controle a execução de sistemas com `.run_if(in_state(AppState::InRoom))`.
- **Linter & Formatação**: Seguir estritamente `cargo fmt` e `cargo clippy` sem avisos (`-D warnings`).
