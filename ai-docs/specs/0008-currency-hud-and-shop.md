# Spec 0008: Moedas no HUD e Loja (bevy_hui)

## Status
Implemented

## Contexto e Objetivo
O jogador vê seus saldos de prata e ouro no HUD, ganha prata jogando e compra itens numa loja de duas colunas. Autoridade e persistência estão na [spec 0005 do servidor](https://github.com/joaovictorsouza/resort-game-server/blob/develop/ai-docs/specs/0005-currency-and-shop.md).

## Escopo
- Dentro: chips de saldo no HUD (ícones Phosphor `coins`), botão "Loja" (`storefront`), janela da loja em `bevy_hui`, heartbeat de recompensas, som ao receber ouro, prévia `examples/shop_preview.rs`.
- Fora: compra de ouro com dinheiro real (emulada por `curl`, ver servidor), desafios, quantidade > 1 por compra, HUD migrado para bevy_hui.

## Componentes Bevy Envolvidos
- `WalletPlugin` (`wallet.rs`): recurso `WalletState`, heartbeat a cada 60 s (e ao entrar na sala), chips `HudSilverText`/`HudGoldText`, `AudioPlayer` com `audio/gold_received.wav` (gerado por `tools/asset_pipeline/generate_coin_sound.py`) quando `delta_gold > 0`.
- `ShopPlugin` (`shop.rs`): recurso `ShopState`, templates `ui/shop.html`, `shop_menu_item.html`, `shop_offer.html` (fontes Tailwind em `assets/ui/src`), funções `shop_close`, `shop_select_category`, `shop_search_changed`, `shop_buy`.
- Mensagens de rede: `NetWalletUpdatedEvent`, `NetShopCatalogLoadedEvent`, `NetPurchaseSucceededEvent`, `NetPurchaseFailedEvent`.
- Notificação Nakama `1001` (`wallet_changed`) tratada em `nakama_client::socket`.

## Comportamento
- Menu lateral com categorias e subcategorias: selecionar uma categoria de topo mostra os itens dela e das subcategorias; só o ramo selecionado expande.
- Busca filtra por nome (sem maiúsculas/acentos) em **todas** as categorias; com busca ativa o destaque do menu é suspenso.
- Cada card mostra miniatura, nome e preço: só prata, só ouro, ou ouro + prata. Botão cinza quando falta saldo; clicar mostra "Saldo insuficiente." sem chamar o servidor.
- Compra: `purchase_item` (quantidade 1); a resposta traz o novo saldo (HUD e cabeçalho atualizam), a linha de feedback confirma e o inventário é recarregado.
- Esc sai do campo de busca e depois fecha; abrir o inventário fecha a loja e vice-versa.

## Critérios de Aceite
- [x] HUD mostra prata e ouro e atualiza sem ação do jogador (heartbeat, compra, concessão via `curl`).
- [x] Botão "Loja" abre janela em duas colunas (menu + grid) com busca.
- [x] Itens só prata, só ouro e ouro + prata aparecem com o preço correto.
- [x] Comprar adiciona o item ao inventário e debita o saldo.
- [x] Receber ouro toca som; gastar ouro não toca.
- [x] Ícones usam Phosphor.
- [x] Lógica pura (menu, filtro, preço, formatação) com testes unitários.
- [ ] `./scripts/check.sh` passa e smoke test com a stack local.
