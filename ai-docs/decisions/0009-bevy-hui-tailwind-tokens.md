# ADR 0009: bevy_hui com Tailwind em build time e ícones Phosphor rasterizados

## Status
Accepted

## Contexto
O cliente usa `bevy_ui` puro, com cores e layout repetidos em cada função de spawn. Queremos templates declarativos com hot reload, design consistente e iconografia padronizada.

## Decisão
- **bevy_hui 0.7** (a versão compatível com Bevy 0.19; a 0.8 exige Bevy 0.20), com a feature `picking`.
- **Tailwind como linguagem de design, não como dependência de runtime.** Não há crate Tailwind compatível com Bevy 0.19 ou bevy_hui. Um gerador Python (`tools/ui/build_ui.py`) lê templates com `class="..."` e escreve os templates do bevy_hui com atributos explícitos. Classe desconhecida é erro. A paleta é a do Tailwind em hex sRGB.
- **bevy_hui_widgets 0.6** (par do bevy_hui 0.7) para o campo de texto: o valor fica no componente `TextInput` e a mudança dispara `on_change`.
- **Phosphor Icons** (MIT, repositório `phosphor-icons/core`) rasterizados para PNG branco e tingidos em runtime com `image_color`. Não existe crate Bevy para Phosphor.
- **Miniaturas** geradas por Blender headless, seguindo o padrão de `tools/asset_pipeline`.

## Consequências
- Hot reload dos templates em desenvolvimento.
- Os templates gerados ficam versionados; editar `assets/ui/src/` e rodar o gerador.
- bevy_hui é um crate pequeno; se o grid for limitado, o fallback é montar os slots em Rust.
- bevy_hui 0.7 não compila estilos dinâmicos (`{prop}`) no nó raiz do template: nós com propriedades em atributos de estilo devem ser filhos de um nó raiz de embrulho.
- Hot reload exige a feature `file_watcher` do Bevy (não habilitada por padrão).
- Atualizar para Bevy 0.20 exige subir bevy_hui para 0.8.
