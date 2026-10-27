# Plano 0001: trocar o pacote de personagem pelo Rukha93 (Modular Anime Character)

Status: **rascunho para decisão** (26/10/2026). Este documento é só planejamento: nenhum código do jogo e nenhum arquivo do pacote foi alterado.
Licenças de pacotes de terceiros: ignoradas por ora, a pedido do dono.
Contexto herdado: [ADR 0007](../decisions/0007-modular-character-pipeline-and-customization.md), [ADR 0010](../decisions/0010-creative-characters-pack-and-single-skeleton.md), [ADR 0011](../decisions/0011-assets-via-cdn.md), [investigação do skinning no Android](../process/investigacao-android-skinning.md), [guia de importação](../process/importar-pacote-de-personagens.md).

## 1. Resumo

- **É viável**, e já existe prova de conceito: `~/Projetos/anime-character-bevy` (Bevy 0.20, repositório git **sem nenhum commit**, só arquivos não rastreados) converte o pacote com Blender (FBX→GLB), porta o `ShaderGraph_CharacterToon` para WESL (`assets/shaders/toon.wesl`), aplica as cores por máscara, o rim light, o outline falso, o normal map e as blend shapes, reaproveita um esqueleto único de 71 juntas e comprime as texturas em `.basis`. Este plano **parte desse protótipo**; o que ele prova e o que ainda não prova está na seção 4.
- **O custo é maior que o do pacote atual**: um avatar completo passa de ~3,2 mil triângulos no corpo e 44 juntas para ~18 mil triângulos e 71 juntas, de ~7 para ~24 draw calls (sem mesclar malhas) e de 1 textura compartilhada (a paleta) para ~12 texturas por visual. O desempenho em S22 (Adreno 730, com o workaround de skinning por uniform buffer) é o maior risco técnico e decide a fase 0.
- **O pacote não traz animações** (nenhum FBX tem clip). Idle, walk, sit e lay precisam ser refeitos para o rig `CC_Base` (Reallusion), com o mesmo método de retarget que já existe no `resort-game-assets`.
- **A personalização cresce muito**: de "1 cor sólida por peça" para "até 6 cores por peça (3 regiões × 2 tons) + 46 blend shapes no rosto + corpo masculino/feminino + 4 rostos por corpo + 22 cabelos, 12 camisetas/casacos, 6 calças, 5 conjuntos, 7 calçados, 9 chapéus, 2 óculos e 3 cachecóis". O protocolo, o servidor, o catálogo, o criador de personagem e o console mudam (quebra de compatibilidade aceita: o projeto não está em produção).
- **Fases**: F0 spike e medições (porta de entrada, decide seguir ou não) → F1 pipeline de assets → F2 material toon e loader no cliente → F3 animações → F4 protocolo e servidor → F5 criador de personagem → F6 console → F7 desempenho e LOD → F8 rollout e limpeza. Estimativa relativa na seção 12.
- **Decisões que são suas** estão na seção 14. As que mais mudam o plano: corpos masculino/feminino de volta, nível de personalização exposto, estilo toon só nos avatares ou também no cenário, e quantos avatares por sala o jogo precisa suportar.

## 2. Inventário do pacote (medido, não estimado)

Origem: `/Users/joaovictor/My project/Assets/Rukha93/ModularAnimeCharacter` (143 MB). O pacote é de Unity **Built-in** (os shaders são `BuiltInTarget`), com dois `.unitypackage` de atualização para URP e HDRP. Documentação: `Documentation.pdf` (1 página de texto, lida na íntegra).

Medições feitas abrindo os 96 FBX no Blender 5.2 em modo headless (script em `scratchpad`, não versionado):

| Grupo | Arquivos | Triângulos por peça | Máx. de influências por vértice | Observação |
|---|---|---|---|---|
| Corpo F e M (`F_body`, `M_body`) | 1 + 1 | 6.274 no total, em **12 submalhas** (`hips`, `torso`, `torso.001`, `torso.002`, `shoulders`, `arms_upper`, `arms_lower`, `hands`, `legs_upper`, `legs_knee`, `legs_lower`, `feet`) | 7 | cada submalha corresponde a um `BodyPartType` do Unity, usado para esconder o corpo sob a roupa |
| Rosto (`F_head.001..004`, `M_head.001..004`) | 4 + 4 | 4.996 (2.676 vértices) | 4 a 6 | **46 blend shapes** (seção 6.3), 6 materiais (rosto, olho, brilho do olho, sobrancelha, cílios, boca) |
| Cabelo (`C_hair_*`) | 22 | 484 a 4.942 | **até 16** | 20 deles têm esqueleto próprio de 4 a 25 ossos (cadeias de cabelo); 2 (`afro.001`, `buzzcut`) não têm esqueleto |
| Chapéu (`C_hat.*`) | 8 | 262 a 2.573 | até 14 | 3 chapéus (`baseballcap`, `beanie`, `racerhelmet`) e 5 "cabelos para usar com chapéu" (`C_hat.hair_*`) |
| Óculos (`C_glasses.*`) | 2 | ~650 | 0 | rígidos, presos à cabeça |
| Cachecol (`C_acc.*`) | 3 | 668 | 7 | |
| Parte de cima (`F_top`, `M_top`) | 12 + 12 | 1.542 a 2.914 | 8 | |
| Parte de baixo (`F_bot`, `M_bot`) | 6 + 6 | 684 a 1.601 | 6 | |
| Conjuntos (`F_outfit`, `M_outfit`) | 5 + 5 | 2.107 a 4.315 | 7 | substituem top + bottom |
| Calçados (`F_shoes`) | 7 | 782 a 1.458 | 4 | só há versão F; o Unity reutiliza no M (decisão aberta 14.2: confirmar) |

- **Esqueleto**: nomes no padrão Reallusion Character Creator (`CC_Base_Hip`, `CC_Base_Pelvis`, `CC_BaseThigh.L`, `CC_BaseThighTwist01.L`...). Os FBX trazem 92 ossos com os "leaf" `_end`; sem eles são **71 juntas**, incluindo 14 ossos de torção (`*Twist01/02`) e os dedos. O corpo atual do jogo tem 44 juntas.
- **Texturas**: 143 MB no pacote inteiro, quase tudo em `Assets/Textures`: `hair` 21 MB (1024²), `tops` 23 MB e `outfits` 10 MB (2048²), `bot` 7,6 MB, `body_F`/`body_M` ~6 MB cada (2048²), `accessories` 2,8 MB, `shoes` 1,8 MB. **Não há textura de cor (albedo)**: cada peça tem `*_RGBMap` (máscara de cores) e `*_Normal`. Eu abri uma máscara: R, G e B marcam regiões (vermelho, amarelo/verde, magenta) com gradientes suaves dentro de cada região, que fazem o papel do sombreamento pintado.
- **Rampa de toon**: `ColorRamp.png`, 512×16.
- **Materiais**: 65 `.mat` mais variações em `Samples/Preview`. Propriedades usadas: `_Color_A_1/A_2/B_1/B_2/C_1/C_2`, `_MaskRemap`, `_RimColor`, `_Outline_Color`, `_Outline_Edge`, `_ColorRamp`, normal adjust. Os óculos usam o shader Standard com transparência (`_ALPHABLEND_ON`).
- **Shaders** (`Assets/Shaders`, Shader Graph, não HLSL escrito à mão):
  - `ShaderGraph_CharacterLit`: PBR com a mesma máscara de cores, metallic/smoothness e fake outline por `Thickness`.
  - `ShaderGraph_CharacterToon`: toon "unlit com uma luz direcional": máscara de cores + rim light.
  - `ShaderGraph_CharacterToon+FakeOutline`: igual, com um outline falso por rim escuro (`Outline Color`, `Outline Thickness`), **sem** passe de casco invertido.
  - `ShaderGraph_ToonBasic`: cor única, para sobrancelha, olho e áreas simples.
  - Subgrafos: `SubGraph_ColorCustomization` (3 regiões × gradiente de 2 cores, `Mask Remap`, `Mask Factor`), `SubGraph_Rimlight`, `SubGraph_ToonShading` (N·L, rampa `ColorRamp`, divisão por 1,7, GI baked, `Normal Adjust` que inclina a normal para (0, −0,5, 1)). O `CustomLighting.hlsl` está vazio ("pode apagar"); a luz principal vem de uma Custom Function que lê `_WorldSpaceLightPos0` e `_LightColor0`.
- **Prefabs**: `F_FullBody`, `M_FullBody` (corpo pelado) e as variantes `AllEquipments` (todas as peças como filhos), mais `Prefabs/HairStyles` (20) e `Prefabs/Accessories` (13), que são rígidos ou com cadeias de ossos e entram como filhos do osso `CC_Base_Head`.
- **Sistema de personalização do sample** (`Samples/Customization`, 1.297 linhas de C#): `CustomizationItemAsset` (peças rigadas, `bodyParts` que a peça esconde, objetos presos a um osso `HumanBodyBones`), `BodyPartTag` (enum com 13 partes), `CustomizationDemo` (equipar = criar `SkinnedMeshRenderer` reaproveitando `bones` e `rootBone` do corpo; trocar cor via `MaterialPropertyBlock` por material; `Equip("outfit")` remove top e bottom). É a especificação de comportamento que o cliente Bevy precisa reproduzir.
- **Animações**: **nenhuma**. O `actions` de todos os 96 FBX é vazio. O sample usa um `Animator` de terceiros.

## 3. O que existe hoje no jogo

- **Pacote atual**: Creative Characters (gratuito), 1 corpo sem gênero (`body_010.glb`, 3.218 triângulos, 1.626 vértices, **44 juntas**, nó raiz `Skeleton`), 30 peças, textura única `characters/palette.png`, tudo em 2,4 MB. Cor por peça pintando `base_color` sólido (`paint_flat`). Catálogo com 7 categorias (`accessory` 13, `top` 5, `bottom` 3, `face` 3, `shoes` 3, `hair` 2, `body` 1).
- **Cliente** (`resortclient`, Bevy 0.20, branch `main` do GitLab): `avatar.rs` (corpo + peças como cenas glTF filhas), `avatar_anim.rs` (um `AnimationPlayer` no nó `Skeleton` do corpo, joints das peças remapeados por nome, `RIG_ROOT_NAME = "Skeleton"`, biblioteca `character_anims.glb` + `clips.json`), `character_creator.rs` (abas, paletas, repintura sem recriar), `appearance.rs` (fallback para peça removida), `cdn_assets.rs` e `catalog_sync.rs` (CDN-only com tiers), `gpu_compat.rs` (workaround Adreno 650/730 que zera storage buffers e força skinning por uniform buffer). Constantes que dependem da altura do corpo: `AVATAR_MODEL_HEIGHT = 1.83`, `LAY_CENTER_OFFSET`.
- **Protocolo** (`crates/protocol/src/messages.rs` e `resort-game-server/internal/protocol/messages.go`): `AvatarAppearance` = `skin_color`, `face_id`, `hair_id`/`hair_color`, `top_id`/`top_color`, `bottom_id`/`bottom_color`, `shoes_id`/`shoes_color`, `accessories: Vec<String>`; sem campo de gênero ou de blend shape; `contract/` com fixtures e `CONTRACT_VERSION`, e `scripts/sync-contract.sh`.
- **Servidor** (`resort-game-server`): catálogo embutido em `data/cosmetics/catalog.json`, `Catalog.Sanitize` ao carregar do Nakama Storage, validação de ids e de cores hexadecimais, e `get_catalogs` para o cliente.
- **Assets e CDN** (`resort-game-assets`, privado): `assets/characters/{base,parts,animations,catalog.json,palette.png}`, `tools/asset_pipeline` (importador, `build_animations.py` com retarget Mixamo, `build_manifest.py` com tiers `core`/`lazy` e `ttl_days`), `scripts/publish-assets.sh` e `verify-cdn.sh`, e o **console web** (`console/`, three.js) com abas de móveis, loja, cabelo/rosto/acessórios, roupas (encaixe automático por transferência de pesos), corpos e gaiola. Todo o console e o importador assumem o esqueleto de 44 juntas.

## 4. O que o protótipo `anime-character-bevy` já prova (e o que não)

Lido por inspeção do código e dos assets; **não executei o protótipo** nem o validei em aparelho.

| Já feito no protótipo | Evidência | Ainda não provado |
|---|---|---|
| FBX→GLB headless com skin e morph targets | `tools/fbx_to_glb.py` (Blender, `export_morph`, sem sparse accessors porque o loader do Bevy rejeita sparse sem bufferView) | limite de 4 influências por vértice e perda de qualidade no cabelo longo |
| Inventário Unity→dados | `tools/inventory.py` (mapa GUID→caminho dos `.meta`, parser de `.mat`, `.asset`, `.prefab`), `export_materials.py` → `materials.json` (67 materiais: 63 `mask`, 4 `flat`) | materiais com transparência (óculos) |
| Shader toon em Bevy 0.20 | `assets/shaders/toon.wesl`: `customize()` com as 3 regiões, rampa por (N·L+1)/2, rim, outline falso, normal por derivadas de tela (sem tangentes), `alpha_cutoff`, `Normal Adjust` | iluminação ligada à luz real da cena (hoje fixa: ambiente 0,7/1,7); sombras recebidas; comparação com render do Unity |
| Cor por peça via máscara | `ToonMaterial`, `ColorOverrides` | **as cores são por nome de material glTF e valem para todos os avatares que o compartilham**: no multiplayer cada avatar precisa de instância própria |
| Esqueleto único: peças reapontadas ao corpo por nome | `check_skeletons.py`, `attach.rs` (cabelo e chapéu pendurados no osso da cabeça), `BODY_PARTS` e ocultação por parte | animações em movimento; deformação em sentar/deitar |
| Blend shapes no Bevy | `MeshMorphWeights`/`MorphWeights` em `lib.rs` | morph targets no Adreno 730 com o workaround de uniform buffer |
| Texturas comprimidas | `tools/texconv` (Basis UASTC/ETC1S), `textures_basis` 70 MB para o pacote inteiro, `bundle_ios.sh`, pasta `build/android` | tamanho real alvo (70 MB é demais; seção 8), tempo de transcode no celular |

## 5. Viabilidade da conversão Unity → Bevy 0.20

| Item Unity | Equivalente no Bevy | Veredito |
|---|---|---|
| Malhas FBX | glTF/GLB via Blender (já feito) | viável |
| Shader Graph (Toon, FakeOutline, Basic) | `Material` customizado (`AsBindGroup`) + fragment em WGSL/WESL, uma única variante com flags (`FLAT`, `HAS_NORMAL`, `HAS_MASK`) | viável (porte pronto no protótipo) |
| `ShaderGraph_CharacterLit` (PBR com máscara) | `ExtendedMaterial<StandardMaterial, …>` ou o toon sem sombreamento | **não portar no início**: o pedido é o estilo toon; decidir se o "lit" é necessário (14.5) |
| Rampa de cores (`ColorRamp.png`) | textura 512×16 amostrada em (N·L+1)/2 | viável; é dado, vai para a CDN |
| Luz principal (`_WorldSpaceLightPos0`) | `lights.directional_lights[0]` | viável; ligar à luz da sala em vez do valor fixo |
| GI baked (ambiente) | `AmbientLight` da cena | viável; calibrar contra captura do Unity |
| Outline falso (rim escuro) | já no fragment, custo zero extra | viável |
| Outline real (casco invertido) | segundo passe/mesh duplicado; **o pacote não usa** | opcional (14.6) |
| Sombra recebida | o fragment atual não amostra o shadow map | decisão 14.7 (cast é gratuito) |
| Blend shapes | morph targets glTF + `MorphWeights` | viável; custo na seção 6.3 |
| Prefab / `SkinnedMeshRenderer` com `bones` copiado do corpo | `SkinnedMesh::joints` remapeado por nome (já faz `rebind_part_joints`) | viável |
| `BodyPartTag` (esconder corpo sob a roupa) | `Visibility::Hidden` por submalha | viável; custo de draw calls (seção 7) |
| `MaterialPropertyBlock` por renderer | material por avatar e por peça (clonar o handle) | viável, **perde batching entre avatares** |
| Hair colliders / `OnChangeEquip` via `SendMessage` | nada equivalente | decisão 14.9 (física do cabelo) |
| Shader do Unity baixado como código | o app só baixa dados (ADR 0011) | o WGSL fica **dentro do app**; só mudanças de dados vão pela CDN |

Riscos de conversão a conferir na F0, em ordem: (1) 4 influências no cabelo/roupa; (2) morph targets no workaround do Adreno; (3) `.basis` no S22 e no iPhone; (4) paridade visual com o Unity; (5) shader compilando em Metal e em Vulkan/Adreno sem os recursos que o `gpu_compat` desliga.

## 6. Rigging, retarget e animações

### 6.1 Esqueleto do projeto novo

- **Esqueleto mestre**: o do corpo `CC_Base`, 71 juntas (92 nós com `_end`). Mantém a regra da ADR 0010: **um `AnimationPlayer` por avatar**, no nó raiz do corpo; roupas e rostos têm os joints remapeados por nome; cabelo e chapéu com esqueleto próprio são pendurados no osso `CC_Base_Head` (como o `attach.rs` do protótipo e o `HumanBodyBones.Head` do sample).
- **Nome da raiz**: hoje o cliente espera `Skeleton` (`RIG_ROOT_NAME`). O Blender exporta o armature como `Armature_F`/`Armature_M`. Padronizar no export para **um nome único** (proposta: `Skeleton`, ficando `Skeleton/RL_BoneRoot/CC_Base_Hip/...`) mantém o contrato do teste `rig_root_name_matches_the_assets`; a escala do FBX (0,01) e a rotação têm de ser aplicadas no export.
- **Ossos de torção** (14) e dedos: o rig humanoide do Unity só anima os ossos humanoides; os de torção ficam no repouso. Em Bevy fazer igual (clips só com os ossos principais) e deixar a torção parada é o caminho barato. Alternativa: descartar os ossos de torção e passar os pesos para o osso pai no export (rig de 57 juntas, menos custo de skinning). A medir na F0; **a escolha muda o `BONE_MAP` e o custo no S22**.
- **Peças sem esqueleto** (óculos, chapéus rígidos, cabelo `afro.001`/`buzzcut` e chapéus rígidos): ficam rígidas e presas a `CC_Base_Head`.
- **Esqueleto "extra" das peças** (cadeias de cabelo): o export mantém só os ossos que têm peso; as cadeias entram como filhas do osso da cabeça e **ficam estáticas** na primeira entrega (ou ganham um pêndulo simples se 14.9 for aprovado).
- **Limite de 4 influências por vértice**: o glTF padrão e o Bevy usam `JOINTS_0`/`WEIGHTS_0` de 4 influências. Cabelo (até 16), chapéus de cabelo (14) e corpo (7) precisam de "Limit Total 4 + Normalize" no export, e o cabelo longo pode perder suavidade. Revisão visual peça a peça na F1.
- **Pivôs/escala**: confirmar que todas as peças usam a mesma pose de repouso (A-pose do Character Creator) e as mesmas inverse bind matrices do corpo; o `check_skeletons.py` do protótipo e o `validate_animations.py` do `resort-game-assets` já fazem esse tipo de conferência e viram teste de regressão.

### 6.2 Animações (nenhuma vem no pacote)

- Necessário no mínimo: `idle`, `walk` (com `tiles_per_sec` e sem root motion), `sit`, `lay`. Reaproveitar o método de `build_animations.py`: Blender só para amostrar, matemática em Python (`rig_math.py`), `BONE_MAP` novo para os nomes `CC_Base`, correção de direção do osso no repouso (A-pose do CC contra T-pose do Mixamo), quadril mantém só o deslocamento vertical escalado pela altura.
- **Fonte dos clips** (decisão 14.8): (a) retargetar `idle`/`walk` do pacote atual (rig de 44 juntas, já em A-pose) para o CC; (b) baixar novas do Mixamo (T-pose) para as quatro; (c) comprar um pacote já em rig Reallusion. A opção (a) mantém o jogo andando igual ao de hoje; (b) amplia emotes.
- Proporção: as peças do Rukha são anime (cabeça grande); o `RETARGET_OFFSETS_DEG` por osso e o `contact_height` (sentar e deitar) vão precisar de ajuste manual, validado pela prévia de render do `render_clip_preview.py`.
- Contrato que o cliente passa a depender: `character_anims.glb` novo (ou `characters/rukha/animations/…`), `clips.json` com os mesmos campos, `AVATAR_MODEL_HEIGHT` e `LAY_CENTER_OFFSET` remedidos.

### 6.3 Blend shapes do rosto

Cada rosto (`F_head.001..004`, `M_head.001..004`) tem **46 shape keys**, agrupadas por prefixos: ajustes (`eye_size`, `nose_length`, `tuck_ears`, `shorten`, `eyelashes_1`), expressões (`happy_01..03`, `angry_01..03`, `sad_01..03`, `surprised_01`, `extra_01`), olhos (`eye_look_*` ×8, `eyes_closed.01..03`, `eyes_wide`, `eye_iris_small`), sobrancelhas (`eyebrows_up/down`), boca (`jaw_open`) e fonemas (`phoneme_*` ×10).

- Custo: o Bevy guarda os deslocamentos de todos os targets numa textura por malha (2.676 vértices × 46 targets, ordem de ~4 MB por rosto, compartilhado entre avatares que usam o mesmo rosto) e o shader só soma os de peso diferente de zero. Confirmar na F0 no iOS e no S22.
- Proposta de escopo (decisão 14.3): na primeira entrega, **só ajustes estáticos** (`eye_size`, `nose_length`, `tuck_ears`, `shorten`) como sliders do criador, mais **piscar** e 3 expressões para emotes. Os fonemas e os `eye_look_*` saem do GLB (menos dados e menos targets por rosto).
- Os morphs vivem **no mesmo GLB do rosto** e são ligados por `MorphWeights` do avatar; o cliente grava os pesos escolhidos na aparência (seção 9).

## 7. Modelo de personalização

### 7.1 Slots, peças e regras

| Slot | Peças (por corpo) | Regras |
|---|---|---|
| `body` (corpo) | 2 (F, M) | decisão 14.1: reintroduzir gênero como `body_type` |
| `face` (rosto) | 4 por corpo | cabeça completa com olhos, boca e sobrancelhas; carrega os morphs |
| `hair` | 22 | cabelos (20 com cadeias de ossos); **com chapéu**, troca por `C_hat.hair_*` correspondente (`buzzcut`, `long`, `medium`, `short`, `tied`) |
| `hat` | 3 | `baseballcap`, `beanie`, `racerhelmet`; ao usar, o cabelo vira a variante `C_hat.hair_*` |
| `glasses` | 2 | |
| `scarf` | 3 | rigado ao corpo |
| `top` | 12 | **esconde** partes do corpo (`bodyParts` do asset) |
| `bottom` | 6 | idem |
| `outfit` | 5 | substitui `top` e `bottom` (regra do `Equip("outfit")`) |
| `shoes` | 7 | |

Regras do sample a reproduzir: equipar `outfit` remove `top` e `bottom` e o inverso; peça equipada atualiza a ocultação das 12 submalhas do corpo; `hat` troca o cabelo pela variante com chapéu.

### 7.2 Cores

- Cada material tem **6 slots**: `A1`, `A2`, `B1`, `B2`, `C1`, `C2` (três regiões da máscara, cada uma com dois tons que a máscara mistura) mais `rim` e `outline` globais. O `Mask Remap` e o `Mask Factor` são parâmetros do material, não do jogador.
- **Níveis de personalização possíveis** (decisão 14.2), do mais barato ao mais caro:
  1. *Simples*: 1 cor por slot de peça (cor base = A1; A2 derivada automaticamente por escurecer/clarear). Paridade com hoje.
  2. *Intermediário*: até 3 cores por peça, uma por região (A, B, C), com A2/B2/C2 derivadas.
  3. *Completo*: os 6 slots por peça, mais os morphs do rosto.
- **Cor da pele**: o corpo do Rukha usa a mesma máscara; qual região da máscara é "pele" e o quanto a pele muda só com A1/A2 **precisa ser confirmado abrindo o `mat_body_*` e o resultado em jogo** (decisão aberta, não assumi).
- **Olhos**: o `mat_eye` usa máscara própria (`mat_eye_RGBMap`), então a cor da íris é uma cor de slot do rosto (A/B/C do olho).
- **Material por avatar**: ao contrário do protótipo (cache por nome do material), cada peça de cada avatar precisa de um `ToonMaterial` próprio, ou de um mecanismo de cor por instância (por exemplo `MeshTag` indexando um buffer de cores). Começar com um material por peça e avaliar o custo na F7.

### 7.3 Como isso vira dados

**Catálogo** (`characters/catalog.json`, versão 3, gerado pelo importador, publicado na CDN e embutido no servidor):

```json
{
  "version": 3,
  "body_types": [{"id": "body_f", "name": "Feminino", "asset_path": "characters/rukha/base/body_f.glb", "height": 1.5}],
  "items": [{
    "id": "top_003_sweater",
    "name": "Suéter",
    "category": "top",
    "body_types": ["body_f", "body_m"],
    "asset_path": {"body_f": "characters/rukha/parts/top/f_top_003_sweater.glb",
                   "body_m": "characters/rukha/parts/top/m_top_003_sweater.glb"},
    "hides": ["torso", "shoulders", "arms_upper"],
    "replaces": ["outfit"],
    "materials": [{"name": "mat_top.003_sweater", "mode": "mask",
                   "mask": "characters/rukha/textures/top_003_sweater.mask.<sha8>.ktx2",
                   "normal": "characters/rukha/textures/top_003_sweater.normal.<sha8>.ktx2",
                   "slots": {"a1": "#...", "a2": "#...", "b1": "#...", "b2": "#...", "c1": "#...", "c2": "#..."},
                   "editable": ["a1", "b1", "c1"]}]
  }],
  "face_morphs": [{"id": "eye_size", "min": 0.0, "max": 1.0}]
}
```

- `editable` diz ao criador e ao servidor **quais slots o jogador pode mudar** nessa peça (peças de uma cor só expõem uma).
- As definições de material saem do `materials.json` global do protótipo e passam para dentro do item (carga preguiçosa junto da peça; um arquivo global com 67 entradas não escala).

**Aparência** (`AvatarAppearance` v2, ver seção 9): ids + cores por slot + pesos de morph quantizados.

**Console** (seção 10): o importador vira um comando que lê o pacote Unity e escreve tudo isso; o console ganha um editor dessas definições.

## 8. Migração dos assets para a CDN

### 8.1 Layout proposto (mesmos princípios da ADR 0011)

```
characters/rukha/base/body_f.<sha8>.glb, body_m.<sha8>.glb      core
characters/rukha/animations/rukha_anims.<sha8>.glb, clips.json  core
characters/rukha/ramp.<sha8>.png                                 core (512x16)
characters/rukha/heads/{f,m}_head_00N.<sha8>.glb                 lazy (ou core só o padrão)
characters/rukha/parts/<categoria>/<id>.<sha8>.glb               lazy, ttl 30 dias
characters/rukha/textures/<id>.{mask,normal}.<sha8>.ktx2         lazy, mesma validade da peça
characters/catalog.json                                           core
ui/thumbs/rukha/<id>.png                                          lazy (gerar com render_item_thumbnails.py)
```

- Hash no nome, `manifest.json` por último, `core`/`lazy` com `ttl_days`: **nada muda no mecanismo**; só se acrescentam prefixos em `CORE_PREFIXES` e `TTL_DAYS_BY_PREFIX` de `build_manifest.py`, e os testes `test_build_manifest.py` ganham os casos.
- O **shader e a rampa**: o WGSL do toon fica dentro do app (só dados vêm da CDN); a rampa é dado e pode ser trocada pela CDN.
- Dependência entre arquivos: cada peça precisa das texturas que ela cita. O `CdnReader` já espera arquivos lazy sob demanda; o carregamento de um item passa a pedir GLB + 2 texturas.

### 8.2 Tamanho e compressão

- Pacote bruto: 143 MB. Protótipo: GLB 36 MB + `.basis` 70 MB para as 96 peças. **Muito para a CDN e, sobretudo, para a memória do aparelho** (seção 11).
- Meta de projeto (a validar na F0/F1): **≤ 350 KB por peça entregue** (GLB + texturas), corpo + rosto ≤ 2,5 MB juntos no tier core.
- Estratégia de texturas:
  - Máscaras: variam suavemente, então **Basis ETC1S** (ou KTX2 com Basis) a 1024² (2048² só no corpo, se a diferença se vir no iPhone). Não usar sRGB.
  - Normais: UASTC (ETC1S arruína normal), a 512²–1024². Avaliar descartar o normal map das peças em que o ganho visual for imperceptível no zoom do jogo (Habbo-like, avatar pequeno); o shader já trata `HAS_NORMAL = 0`.
  - Gerar mipmaps no `texconv`; atlas de rosto não é necessário.
- Estratégia de malhas: sem Draco (o Bevy não lê); avaliar `KHR_mesh_quantization` e `EXT_meshopt_compression` suportados pelo loader do Bevy 0.20 (a confirmar na F0); remover do GLB os atributos que o shader não usa (tangentes, `COLOR_1`, segundo UV) e os morph targets que o produto não usa (fonemas).
- Total esperado após otimização (estimativa a confirmar): corpo F+M ~3 MB, ~100 peças lazy ~20–35 MB, 100 miniaturas ~1 MB. Cada aparelho baixa só o que o jogador vê.
- Cache e versões: igual à ADR 0011 (sha256, tamanho, gravação atômica, `prune_expired`). Rollback = republicar um manifest anterior. Publicação **sempre manual** (`publish-assets.sh dev` e depois `--apply`), com `verify-cdn.sh` na sequência.

### 8.3 Pipeline de importação

Novo `tools/asset_pipeline/import_rukha_pack.py` (ou o `anime-character-bevy/tools` movido para o `resort-game-assets`), em etapas reexecutáveis e testadas:

1. `inventory.py` (GUID→caminho, materiais, assets de personalização, prefabs).
2. FBX→GLB no Blender headless (ajustar escala 0,01, raiz `Skeleton`, 4 influências, remover extras, manter morphs úteis).
3. Mesclar as 12 submalhas do corpo conforme a decisão de draw calls (seção 11).
4. Exportar materiais por item (`materials[]` do catálogo).
5. Converter texturas (`texconv`, ETC1S/UASTC, mipmaps).
6. Gerar `catalog.json` v3 e as cópias do servidor (`resort-game-server/data/cosmetics/catalog.json`).
7. Validar (`validate_animations.py` adaptado, `test_assets_consistency.py` com os nomes `CC_Base`, tamanhos máximos por peça).
8. Miniaturas (`render_item_thumbnails.py`).
9. `build_manifest.py`, `publish-assets.sh` (dry-run).

Os arquivos do pacote Unity **não entram no repositório** (pasta `Temp Package/Rukha93/` ignorada, como hoje).

## 9. Impacto no protocolo, no servidor e nos dados salvos

Quebra de compatibilidade aceita (projeto não está em produção). Mudanças:

- **`AvatarAppearance` v2** (cliente `messages.rs` e servidor `messages.go`, mais fixtures em `contract/`, `CONTRACT_VERSION` incrementado e `scripts/sync-contract.sh`/`check-contract.sh`):
  ```json
  {"body_type": "body_f", "face_id": "f_head_002",
   "face_morphs": {"eye_size": 128, "nose_length": 40},
   "slots": {"hair": {"id": "hair_long_001", "colors": ["#..", "#.."]},
             "top": {"id": "top_003_sweater", "colors": ["#.."]}},
   "accessories": ["glasses_01_square", "scarf_001"], "skin": ["#..", "#.."]}
  ```
  Cores como lista por slot (ordem `a1,a2,b1,b2,c1,c2` truncada pelo `editable`); morphs como inteiros 0–255; objeto bem menor que 600 bytes, aceitável nos snapshots de sala (`RoomSnapshot`/`AvatarJoined`).
- **Servidor**: `Catalog` v3, `Sanitize` que valida `body_type`, `face_id`, ids por categoria e por `body_types`, regra de `outfit` contra `top`/`bottom`, cores só nos slots `editable`, morphs dentro do intervalo, e que `hair` sem `hat` compatível cai no padrão. Testes Go (`go test -race ./...`) e `TestDefaultAppearanceIsInEmbeddedCatalog` ajustados. `DefaultAppearance()` e `AvatarAppearance::default()` apontam para itens do catálogo novo.
- **Dados salvos**: aparências antigas viram o visual padrão (as cores de pele, cabelo e roupas podem ser mapeadas para o A1 do slot correspondente); migração no `Sanitize`, sem script; o cliente continua mapeando ids desconhecidos para o primeiro item da categoria (`resolve_part_id`).
- **`get_catalogs`**: o catálogo v3 é maior (materiais por item); medir e, se preciso, paginar por categoria ou enviar só o delta (`get_catalogs_unchanged` já existe).
- **Console → servidor**: o "Exportar" do console já gera branch no clone do servidor com `go test`; o item novo entra como `draft` (regra existente).
- **Cliente**: `appearance.rs` e `character_creator.rs` trocam `paint_flat` por cores por slot e material por peça; `avatar.rs` ganha `body_type` e a ocultação por submalha.

## 10. Console de assets

O que já existe presume 44 juntas e um corpo só. Impacto:

| Aba atual | O que muda |
|---|---|
| Roupas (alinhar ao corpo, transferir pesos, penetração por pose) | **deixa de ser o fluxo principal**: as peças do Rukha já vêm rigadas no mesmo esqueleto. Mantida para roupas de terceiros, mas exige generalizar `skeleton.py` e `fitting.py` para o esqueleto `CC_Base` e para dois corpos |
| Cabelo/rosto/acessórios (gizmo na cabeça, rig animado) | precisa carregar cadeias de ossos do cabelo e os morph targets do rosto; ajuste de posição continua útil |
| Corpos e gaiola | `bodies.py` valida altura e esqueleto contra **um** corpo base; passa a validar contra F e M (decisão 14.1) |
| Móveis e loja | sem mudança |
| **Nova: Pacote Rukha93** | importar o pacote e rodar o pipeline da 8.3 com relatório (peças, tamanhos, 4 influências, texturas) |
| **Nova: Materiais** | editor dos slots `a1..c2`, `editable`, `Mask Remap`, rim e outline com a prévia **usando o mesmo shader toon**. A prévia hoje usa three.js; duas opções: portar o shader para GLSL (risco de divergir do jogo) ou renderizar a prévia pelo exemplo `creator_preview` do cliente (fonte única de verdade, mais lento) |

Sem mexer no console na primeira entrega: usar só o importador por linha de comando (F1) e o criador do cliente para validar visualmente; o console entra na F6.

## 11. Desempenho em iPhone 16 Pro e Galaxy S22 (Adreno 730)

Números **por avatar completo** (corpo + rosto + cabelo + top + bottom + shoes), estimados com as medições da seção 2:

| | Atual | Rukha93 (sem otimizar) | Meta após F7 |
|---|---|---|---|
| Triângulos | ~5 mil | ~18 mil | ≤ 10 mil (LOD de longe: ≤ 3 mil) |
| Juntas | 44 | 71 | 57–71 |
| Draw calls (primitivas) | ~7 | ~24 (12 do corpo + 6 do rosto + 6 das peças) | ≤ 10 |
| Texturas | 1 compartilhada | ~12 distintas (máscara + normal por peça) | ≤ 6 e 1024² |
| Material | 1 PBR | 1 por peça e por avatar | idem, com `MeshTag` se preciso |

Pontos de atenção, em ordem de risco:

1. **S22 (Adreno 730) com `gpu_compat`**: sem storage buffers o skinning usa uniform buffer e o preprocessing roda na CPU; com 71 juntas e ~24 draw calls por avatar, o gargalo provável é CPU (preparo de draw calls e skinning), não a GPU. Medir antes de decidir qualquer coisa. Morph targets e `.basis` precisam ser testados **nesse** modo.
2. **Draw calls do corpo**: as 12 submalhas existem só para esconder partes sob a roupa. Alternativas: (a) mesclar em 3–4 grupos e usar variantes pré-mescladas por combinação de roupa (explosão combinatória), (b) manter 12 e mesclar só para LOD distante, (c) mesclar o corpo em uma malha e esconder por **máscara por vértice** no shader (um atributo/UV com o id da parte e um bitmask de partes escondidas no material). A (c) dá 1 draw call e não depende de combinações; é a recomendada, mas é a mudança de maior risco no shader.
3. **Memória de texturas**: uma 2048² comprimida a 4 bpp com mipmaps ≈ 2,7 MB; um visual completo de ~12 texturas ≈ 15–25 MB de VRAM, e cada visual distinto na sala soma. Com 20 jogadores todos diferentes, o orçamento estoura sem reduzir para 1024² (÷4) e sem descarregar o que saiu da sala.
4. **Custo do shader**: o fragment tem 2 amostras de máscara, 1 de normal, 1 de rampa, derivadas para o normal e vários `smoothstep`; é barato para o tamanho de avatar de um jogo de sala isométrica, mas o `discard` do cabelo (`alpha_cutoff`) desliga early-z; medir.
5. **LOD**: gerar LOD1 no Blender (decimação do corpo para ≤ 2,5 mil, rosto sem morphs, sem normal map) e trocar por distância/tamanho na tela. Política depende da decisão 14.4 (quantos avatares por sala).
6. **Orçamento proposto para aprovar** (a validar com medição): iPhone 16 Pro ≥ 60 fps e S22 ≥ 45 fps com 20 avatares distintos visíveis, 1,5 GB de pico de RAM e ≤ 300 MB de VRAM do jogo; abaixo disso, a F7 reduz resolução, ativa LOD e mescla malhas antes de qualquer corte de conteúdo.

## 12. Fases, critérios de pronto e estimativa relativa

Estimativa em tamanhos relativos (P ≈ 1–2 dias, M ≈ 3–5, G ≈ 1–2 semanas, GG ≈ 2+ semanas), de uma pessoa trabalhando com o apoio de agentes.

| Fase | Entrega | Pronto quando | Tamanho |
|---|---|---|---|
| **F0 Spike e medições** | Protótipo commitado (em `resort-game-assets/tools/` ou repo próprio), build no iPhone 16 Pro e no S22 com o `gpu_compat` do cliente, relatório de medições | (1) FPS e frame time com 1, 10 e 20 avatares nos dois aparelhos; (2) morph targets e `.basis` funcionando no Adreno 730 com uniform skinning; (3) altura, A-pose e `RETARGET` medidos; (4) comparação lado a lado com captura do Unity; (5) decisão **seguir/ajustar/parar** escrita neste documento | M |
| **F1 Pipeline de assets** | `import_rukha_pack.py` + testes, catálogo v3, texturas comprimidas, GLBs limpos | um comando gera `assets/characters/rukha/**` do pacote; todas as peças com ≤ 4 influências, raiz `Skeleton`, mesmos nomes de junta; tamanho por peça dentro da meta; `./scripts/check.sh` verde; publicável no DEV em dry-run | G |
| **F2 Material toon e loader** (cliente) | `ToonMaterial` e o shader no `crates/game`, carregamento de GLB + texturas pela CDN, material por peça e por avatar, ocultação das partes do corpo | um avatar vestido renderiza igual ao Unity (diferença visual aprovada em captura), cores por slot mudam sem recriar, sem regressão de `cargo test`/`clippy` | G |
| **F3 Animações** | `rukha_anims.glb` e `clips.json` (idle, walk, sit, lay) | os quatro clips sem interpenetração relevante (relatório de penetração do console e conferência visual), `tiles_per_sec` calibrado, `validate_animations.py` verde | G |
| **F4 Protocolo e servidor** | `AvatarAppearance` v2, `Catalog` v3, `Sanitize`, fixtures, `CONTRACT_VERSION` | `go test -race ./...` e `scripts/check-contract.sh` verdes; aparência antiga abre sem erro; dois clientes veem o mesmo visual | M |
| **F5 Criador de personagem** | abas por slot, seletor de cores por região, sliders do rosto, escolha de corpo, prévia animada | criar, salvar e reabrir o personagem no iOS e no Android; outro jogador vê o resultado; testes de `appearance.rs` e do estado do criador | G |
| **F6 Console** | aba Pacote Rukha93 e aba Materiais, `skeleton.py` generalizado | importar o pacote e editar materiais pelo console reproduz o resultado da linha de comando | G |
| **F7 Desempenho e LOD** | mesclagem de malhas, LOD, texturas 1024², orçamento de memória | metas da seção 11 atingidas nos dois aparelhos com 20 avatares | G |
| **F8 Rollout e limpeza** | publicação DEV, `verify-cdn.sh`, teste em aparelho, ADR 0015, remover o pacote Creative Characters e o código morto, atualizar `importar-pacote-de-personagens.md` | app novo no DEV com um fluxo completo (login → criador → sala com 2 jogadores), docs atualizados, caminhos antigos removidos | M |

Dependências: F0 antes de tudo; F1 e F4 em paralelo; F2 depende de F1; F3 de F1 (e do rig definido na F0); F5 de F2, F3 e F4; F6 e F7 depois de F5; F8 por último. Caminho crítico: F0 → F1 → F2 → F5. O primeiro marco visível (avatar do Rukha andando numa sala, sem criador) fica no fim da F3.

## 13. Riscos

| Risco | Probabilidade | Impacto | Mitigação |
|---|---|---|---|
| Desempenho no S22 abaixo do orçamento (CPU, 71 juntas, ~24 draw calls/avatar) | alta | alto | medir na F0; mesclar malhas (11.2c), LOD, cortar ossos de torção |
| 4 influências estragam cabelo longo e roupas largas | média | médio | revisão peça a peça na F1; reponderar no Blender; excluir peças ruins |
| Morph targets ou `.basis` falham no Adreno com o workaround | média | alto | testar na F0; plano B sem morphs (rostos fixos) e KTX2 |
| Cores por material compartilhado vazam entre avatares (como no protótipo) | certa se não tratada | alto | material por avatar e peça desde a F2; teste automatizado com dois avatares |
| Diferença visual contra o Unity (ambiente 0,7 fixo, ausência de GI) | média | médio | captura de referência com o Unity instalado; calibração do ambiente |
| Retarget das animações em proporção anime (sentar, deitar) | média | médio | `contact_height` medido, `RETARGET_OFFSETS_DEG`, relatório de penetração |
| Texturas pesadas na memória e na CDN | alta | médio | 1024², ETC1S nas máscaras, descarregar por TTL, orçamento de VRAM |
| Console desatualizado depois da troca (esqueleto de 44 juntas) | certa | baixo/médio | F6 explícita; na meantime importação só por CLI |
| Prototype ainda sem commit e dependente de caminhos locais | certa | médio | commitar/mover para `resort-game-assets` na F0 |
| Catálogo v3 grande demais para `get_catalogs` | baixa | baixo | medir; delta e paginação |
| Mudança do Bevy (bug #24926 corrigido ou não) | média | baixo | manter `gpu_compat` até a correção; não depender de storage buffers |

## 14. Decisões abertas (para o dono decidir)

Cada item traz minha recomendação; **não assumi nenhuma** nos números acima, exceto onde dito.

1. **Corpo masculino e feminino de volta?** O pacote tem corpos, rostos, roupas e conjuntos separados por F/M; a ADR 0010 tinha removido o gênero. *Recomendo* voltar com `body_type` (tem custo em protocolo, console e catálogo, mas é a estrutura do pacote).
2. **Nível de personalização exposto** (7.2): simples (1 cor), intermediário (3 cores) ou completo (6 cores + morphs). *Recomendo* intermediário na primeira entrega e completo depois, sem mudar o protocolo (as listas de cores já comportam).
3. **Escopo das blend shapes**: só ajustes estáticos e piscar, ou emotes e fala? *Recomendo* ajustes estáticos + piscar + 3 expressões; cortar fonemas e olhar.
4. **Quantos avatares por sala** o jogo precisa suportar e qual a política de LOD? Define o orçamento da seção 11.
5. **Estilo toon só nos avatares ou também no cenário?** Mobília e salas são PBR; avatares toon podem destoar. *Recomendo* validar com 3 salas na F0 antes de decidir toonificar móveis.
6. **Outline real (casco invertido)** ou só o "fake outline" do pacote? *Recomendo* só o do pacote.
7. **Sombras**: avatares projetam sombra (barato) e recebem (exige mudança no shader)? *Recomendo* projetar, e receber só se a F0 mostrar necessidade.
8. **Fonte das animações**: retargetar as atuais, baixar do Mixamo ou comprar? (6.2)
9. **Física do cabelo**: cadeias de ossos estáticas ou um pêndulo simples? *Recomendo* estáticas na primeira entrega.
10. **Calçados**: o pacote só tem versão `F_shoes`; confirmar se o `M` usa a mesma malha com o mesmo encaixe (preciso abrir no Unity, não assumi).
11. **Resolução de textura no mobile** (1024² para tudo ou 2048² no corpo) e se o normal map entra.
12. **Cor da pele**: qual região/slot do material do corpo é a pele e quantos tons o jogador escolhe (7.2).
13. **Manter o pacote Creative Characters em paralelo** durante a transição (flag) ou trocar de vez? *Recomendo* trocar de vez na F8; o projeto não está em produção.
14. **Destino do protótipo `anime-character-bevy`**: mover o código útil para `resort-game-assets/tools` e `resortclient/crates` e arquivar o resto.
15. **Se o material `Lit` (PBR com máscara) entra** (5): *recomendo* não portar.

## 15. Fora do escopo deste plano

Implementação de qualquer fase, mudanças no jogo, no servidor, nos assets publicados ou no pacote Rukha93, e licenciamento dos pacotes de terceiros.
