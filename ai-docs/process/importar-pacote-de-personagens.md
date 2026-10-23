# Importar ou trocar o pacote de personagens

Guia para substituir os personagens, as roupas e as animações (por exemplo, ao passar do pacote gratuito para o completo). Contexto e decisões: [ADR 0010](../decisions/0010-creative-characters-pack-and-single-skeleton.md).

Tudo roda na raiz do repositório, em Python 3 sem dependências. O Blender só é usado para ler os FBX do Mixamo.

## Pré-requisitos

- Pacote extraído em `Temp Package/<nome>/` (a pasta é ignorada pelo git), com um GLB de personagem completo e uma pasta de peças separadas, todas sobre o mesmo esqueleto.
- Os FBX do Mixamo das poses (ex.: `Sitting.fbx`, `Lying Down.fbx`) em `Temp Package/Mixamo/`, exportados com a skin, 60 fps.
- Blender instalado (`/Applications/Blender.app`, ou `--blender <caminho>`).

## Passo a passo

1. **Ajuste os nomes em `tools/asset_pipeline/anim_config.py`**, se o pacote for diferente: `PACK_CHARACTER`, `PACK_PARTS_DIR`, `PACK_BODY`, `PACK_PALETTE`, `MODEL_HEIGHT` (altura do corpo, o `build_animations.py` imprime o valor medido), `PACK_CLIPS` (nome das animações do pacote usadas para `idle` e `walk`) e `MIXAMO_POSES`.
2. **Classifique as peças em `import_character_pack.py`**: `PREFIX_CATEGORY` mapeia o prefixo do nome do arquivo para a categoria (`face`, `hair`, `top`, `bottom`, `shoes`, `accessory`). Um prefixo desconhecido interrompe a importação, de propósito. Se um tipo de peça novo precisar de uma aba no criador, ele exige código em `character_creator.rs`.
3. **Importe as peças** (apaga `assets/characters/base` e `parts`):
   ```bash
   python3 tools/asset_pipeline/import_character_pack.py --pack "Temp Package/Creative_Characters"
   ```
   Isso também regrava `catalog.json` no cliente e no servidor. O `tint` de cada item é calculado pela faixa da paleta que a malha usa; revise a lista "Tintable" impressa.
4. **Gere as animações**:
   ```bash
   python3 tools/asset_pipeline/build_animations.py --pack "Temp Package/Creative_Characters" --mixamo "Temp Package/Mixamo"
   ```
   O script confere o skinning no rest pose, escreve `character_anims.glb` e `clips.json` e imprime `contact` e `body middle along Z` de cada pose.
5. **Atualize as constantes do cliente** com o que foi medido: `AVATAR_MODEL_HEIGHT` e `LAY_CENTER_OFFSET` em `avatar.rs`, e os ids padrão (`AvatarAppearance::default()` em `protocol/src/messages.rs`, `DefaultAppearance()` em `resort-game-server/internal/protocol/messages.go`). Os testes `the_shipped_catalog_has_the_defaults_and_every_slot` (cliente) e `TestDefaultAppearanceIsInEmbeddedCatalog` (servidor) falham se um padrão não existir no catálogo.
6. **Valide**:
   ```bash
   python3 tools/asset_pipeline/validate_animations.py
   python3 -m unittest discover -s tools/asset_pipeline
   (cd server && go test -race ./...)
   (./scripts/check.sh)
   ```
7. **Confira visualmente**: `cargo run -p game --example creator_preview -- shot.png skin sit` salva um screenshot do criador com a pose (`idle`, `walk`, `sit` ou `lay`). Para ver os clips fora do motor, `build_animations.py ... --preview-glb /tmp/preview.glb` e depois `Blender -b --python tools/asset_pipeline/render_clip_preview.py -- /tmp/preview.glb /tmp/render`.

## Quando o pacote novo é diferente

- **Esqueleto com outros nomes de joint**: `BONE_MAP` em `anim_config.py` liga cada joint do pacote ao osso do Mixamo (e ao osso filho, para a correção de direção). Ajuste a tabela; o teste `test_anim_config.py` garante simetria esquerda/direita e a ordem pai antes do filho.
- **Nó raiz da cena com outro nome**: troque `RIG_ROOT_NAME` em `avatar_anim.rs` (o teste `rig_root_name_matches_the_assets` aponta a divergência).
- **Vários corpos ou gêneros**: hoje há um corpo fixo (`BODY_FILE` em `avatar.rs`). Escolher o corpo exige um campo novo em `AvatarAppearance` (skill `protocol-change`).
- **Mais animações do pacote** (correr, pular, emotes): inclua em `PACK_CLIPS` e use o nome em um `AvatarPose` novo.
- **Texturas diferentes da paleta única**: o importador falha se o atlas embutido diferir de `PACK_PALETTE`. Pacotes com texturas por peça pedem outro tratamento de cor (hoje `paint_flat` descarta a textura).

## Aparências já salvas

Ao trocar o catálogo, as aparências salvas com ids que não existem mais não quebram: o servidor as sanitiza ao carregar e o cliente usa o primeiro item da categoria. As cores são mantidas.
