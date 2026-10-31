# Pipeline dos personagens toon (pacote Rukha93)

Como gerar os assets que a crate `toon_avatar` e o jogo leem, e como ver o resultado. Decisões: [ADR 0015](../decisions/0015-crate-de-personagens-toon.md). Os arquivos do pacote Unity e os GLB gerados **não entram neste repositório** (licença limitada): ficam numa pasta de trabalho e, depois de prontos, no `resort-game-assets` e na CDN (publicação manual, ver [ADR 0011](../decisions/0011-assets-via-cdn.md)).

## Pré-requisitos
Blender (`/Applications/Blender.app`), Python 3 e o pacote Unity extraído. A conversão FBX→GLB, os LODs, as texturas compactadas (`texconv`), o atlas de pele e rosto e os catálogos `outfits.json`/`attachments.json`/`materials.json` vêm do protótipo `anime-character-bevy` (`tools/`: `inventory.py`, `fbx_to_glb.py`, `export_*.py`, `make_lods.py`, `fit_garment.py`, `texconv`). Esse protótipo ainda não está num repositório; a F8 do plano move essas ferramentas para o `resort-game-assets`.

## O que esta crate gera (`crates/toon_avatar/tools`)
```bash
# 1. Animações (precisa dos FBX do Mixamo: caminhada, sentar, deitar)
python3 -I crates/toon_avatar/tools/animations/build_cc_animations.py \
  --body <assets>/characters/original/F_body.glb --head <assets>/characters/original/F_head.001.glb \
  --walk Walking_Girl.fbx --sit Sitting.fbx --lay "Lying Down.fbx" --out <assets>/characters/animations/F
# (repita com M_body.glb, M_head.001.glb e o caminhar masculino, saída em .../animations/M)

# 2. Objetos de demonstração (copo e lata)
Blender -b --python crates/toon_avatar/tools/props/make_demo_props.py -- <assets>/characters/props

# 3. Catálogos: o do servidor (validação) e os do cliente (`avatar_catalog.json`, `materials.json`)
python3 -I crates/toon_avatar/tools/catalog/export_server_catalog.py --items <assets>/characters/items \
  --out <servidor>/data/cosmetics/catalog.json --client-out <assets>/characters --materials <assets>/characters/materials
```
O `clips.json` de cada corpo traz a duração, a altura de contato, a velocidade de caminhada e a pegada da mão direita. Para trocar as animações (por exemplo, baixar um idle e um gesto de beber do Mixamo), rode o passo 1 de novo.

## Como o jogo encontra os arquivos
`characters/avatar_catalog.json`, `characters/materials.json`, `characters/animations/{F,M}/{character_anims.glb,clips.json}`, `characters/props/<id>.glb`, os GLB em `characters/{original,models,lod1,lod2}` e as texturas em `characters/textures` (desktop) ou `characters/textures_basis/<512|1024|2048>` (mobile). Tudo passa pelo `AssetServer` (e portanto pela CDN).

## Ver o resultado sem servidor
```bash
# demonstrador (a tela de montagem do protótipo)
TOON_AVATAR_ASSETS=<assets> cargo run -p toon_avatar_demo
#   teclas: espaço top, B bottom, S sapato, U conjunto, H rosto, X corpo, N cabelo, Y chapéu, G óculos,
#   P pose, F expressão, V objeto, C beber, K cenário cartoon, O outline, M mesclagem, Q tier
#   captura: TOON_DEMO_SNAPSHOT=out.png TOON_DEMO_POSE=walk|sit|lay TOON_DEMO_EXPR=happy TOON_DEMO_HOLD=cup ...

# criador de personagem do jogo (precisa de $BEVY_ASSET_ROOT/assets com ui, fonts e characters)
COSMETICS_JSON=<servidor>/data/cosmetics/catalog.json cargo run -p game --example creator_preview
```

## O que falta
Compressão e tamanhos por peça dentro do orçamento, manifesto com os campos `lod`/`min_tier`, publicação no DEV e `verify-cdn`, console de assets adaptado ao esqueleto `CC_Base` e a validação em aparelho (iPhone, S22) com medições de quadros.
