# ADR 0002: Cliente Nakama Customizado em Rust

## Status
Aceita

## Contexto
Não existe SDK oficial em Rust mantido pela Heroic Labs para Nakama. O crate comunitário `nakama-rs` encontra-se sem manutenção desde 2021 (versão 0.1.1).

## Decisão
Implementar um crate dedicado e leve (`crates/nakama_client`) utilizando `reqwest` para requisições REST (autenticação de device e RPCs) e `tokio-tungstenite` para a camada WebSocket em tempo real. O cliente executa dentro de uma thread nativa gerenciada por um runtime Tokio, comunicando-se com o Bevy via canais `crossbeam_channel`.

## Consequências
- Controle absoluto do protocolo e do ciclo de vida da conexão.
- Desacoplamento limpo: o motor gráfico nunca é bloqueado por latência de rede.
