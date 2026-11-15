# Spec 0009: Ajustes de UI para mobile (HUD e telas HTML)

## Status
Proposed (plano simplificado; nada implementado)

## Objetivo
Telas `bevy_hui` e HUD legíveis e tocáveis em iPhone e Android, com o mínimo de mudança. Sem crate nova, sem breakpoints em runtime, sem trocar o gerador Tailwind (que já aceita valores arbitrários como `w-[95vw]`). Responsividade completa (`sm:`/`md:`) fica como evolução futura, só se estes ajustes não bastarem.

## Causa principal
`ui_scale.rs` **encolhe** a UI para caber: num iPhone 16 em paisagem (852x393 pt) o fator é 0,655, então `text-xs` (12 px) vira ~8 pt. Além disso o HUD (`wallet.rs`: ícone 18 px, texto 14 px) usa `Val::Px` fixo e é pequeno demais para toque.

## Mudanças

### 1. Parar de encolher (`ui_scale.rs`)
- Em toque (Android/iOS), `UiScale` = 1,0 (talvez 1,15 após teste no aparelho); desktop continua 1,0. Nunca abaixo de 1.
- Substituir o teste `phone_in_landscape_shrinks_to_fit_the_reference_layout` por testes da nova política.

### 2. Modais que se ajustam à tela (templates em `assets/ui/src`)
Trocar tamanhos fixos por limites relativos à janela:
- Janela da loja/inventário/navegador/criador: `w-[95vw] max-w-[900px] h-[92vh] max-h-[560px]` (em celular quase tela cheia; no desktop continua 900x560).
- Menu lateral da loja: largura menor (ex. `w-[160px]`) e `text-sm`; grid com `flex-wrap` e slots de largura fixa menor, deixando o número de colunas variar com a largura (já é assim no inventário).
- Texto mínimo `text-sm` (14) nas telas; botões e itens clicáveis com altura ≥ 44 (`h-11`/`min-h-11`).
- Verificar na prática se `vw`/`vh` do hui respeitam o `UiScale`; se não, ajustar a conta (ou usar `%` num pai de tela cheia).
- Rodar `python3 tools/ui/build_ui.py` e conferir com `--check`.

### 3. HUD (`wallet.rs` e botões do HUD)
- Chips de moeda: ícone 18 -> 24, texto 14 -> 18, padding maior.
- Botões do HUD (Loja, Inventário, Navegador): alvo de toque ≥ 44x44, ícones ≥ 24, `gap` maior.
- Se for simples, usar constantes únicas (`HUD_ICON`, `HUD_TEXT`, `HUD_TOUCH`) em vez de números espalhados. Migrar o HUD para templates hui só se o ganho for claro (fica fora por padrão).

### 4. Verificação
- `examples/*_preview.rs` com a janela forçada para 852x393, 667x375 e 915x412 (paisagem) e um retrato (390x844); capturas antes/depois.
- Testar em iPhone (`scripts/run-ios-sim.sh`) e Android físico (`scripts/run-android.sh`).
- `./scripts/check.sh` verde.

## Critérios de Aceite
- [ ] Nenhum texto menor que 14 pt em celular; botões e chips tocáveis ≥ 44 pt.
- [ ] Loja, inventário, navegador, criador e campainha cabem sem corte em paisagem 667x375 até 915x412 e em retrato 390x844.
- [ ] HUD legível em iPhone e Android; desktop sem regressão visual.

## Riscos
- Em retrato muito estreito alguns layouts de duas colunas (loja) podem ficar apertados; se acontecer, avaliar variantes de layout (evolução futura).
- Área segura (notch) não tratada; avaliar se corta o HUD nos aparelhos reais.

## Estimativa
1-2 dias, a maior parte em ajustar e olhar cada tela nos tamanhos de referência.

> Nota (25/10/2026): os scripts e projetos nativos citados aqui (`run-android.sh`, `ios/`, etc.) foram removidos; veja o [ADR 0014](../decisions/0014-remocao-do-mobile-nativo-antigo.md).
