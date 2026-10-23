# Arquitetura de Rede: Tokio ↔ Bevy ECS

## Desafio
O motor gráfico Bevy roda seu próprio scheduler concorrente em threads nativas por frame. Não é recomendado bloquear o loop do Bevy com chamadas assíncronas de rede ou executar WebSockets pesados diretamente nas threads de renderização.

## Solução com Background Worker
1. Ao iniciar o jogo, o `NetPlugin` instancia `NakamaClientHandle::spawn()`.
2. Essa chamada cria uma thread dedicada chamada `nakama-net-worker` contendo um runtime `tokio` multi-thread.
3. A comunicação com o Bevy é feita através de dois canais `crossbeam_channel`:
   - `Sender<NakamaCommand>`: Bevy envia comandos (ex: `SendMessage(ClientMessage)`).
   - `Receiver<NakamaEvent>`: Tokio entrega eventos e mensagens recebidas.
4. A cada frame do Bevy, o sistema `poll_nakama_events` consome de forma não-bloqueante (`try_recv()`) os eventos pendentes e despacha eventos no ECS do Bevy.
