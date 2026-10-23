# Spec 0006: Poses e Animações do Avatar

## Status
Implemented. Revisada em 2026-10-21 ([ADR 0010](../decisions/0010-creative-characters-pack-and-single-skeleton.md)): pacote *Creative Characters*, esqueleto `Skeleton` de 44 joints, sem gênero, só as poses `Sit` e `Lay`; o texto abaixo já reflete isso.

Histórico: Implemented (fases 1–3). A fase 4 (postura autoritativa no servidor) está na [spec do servidor 0004](https://github.com/joaovictorsouza/resort-game-server/blob/develop/ai-docs/specs/0004-avatar-posture-and-seats.md): o cliente aplica `AvatarPostureChanged` em `AvatarPosture` (`sit` → `Sit`, `lay` → `Lay`) e a rotação 0..7 no yaw do avatar.

## Contexto e Objetivo
O avatar aparece, sem animação, na pose de repouso do rig (T-pose). Esta spec define:

- idle quando o avatar está parado;
- ciclo de caminhada enquanto ele segue o caminho do servidor;
- poses de sentar e deitar, prontas para os móveis com `behavior_id = "seat"` / `"bed"`.

## Escopo
- **Dentro do escopo**:
  - Pipeline `tools/asset_pipeline/build_animations.py` que gera `assets/characters/animations/character_anims.glb` e `clips.json`.
  - `idle` e `walk` vêm das animações do pacote; `sit` e `lay` são uma pose do Mixamo (`Sitting.fbx`, `Lying Down.fbx`) retargetada para o esqueleto do pacote.
  - Esqueleto único por avatar: o corpo recebe o `AnimationPlayer` e as peças cosméticas reaproveitam os joints do corpo.
  - Máquina de estados de pose no cliente e atalho de debug (`P`, ou `fn`+F6 no Mac) para testar sit/lay.
- **FORA do escopo**:
  - Sentar no chão, deitar no sofá, emotes e transições animadas (sentar → levantar). Entram quando houver animações para eles.

## Requisitos Funcionais e Técnicos

### Clips (`clips.json`)
| Clip | Fonte | Loop | Uso |
|---|---|---|---|
| `idle` | `Idle_Relaxed` do pacote | sim | Parado |
| `walk` | `Walk_Forward` do pacote, sem movimento para a frente do `Root` | sim | Andando |
| `sit` | Último frame do `Sitting.fbx` (Mixamo) | sim | Assento |
| `lay` | Último frame do `Lying Down.fbx` (Mixamo), de costas | sim | Cama |

Campos de cada clip: `loop`, `duration_s`, `contact_height` (altura, em unidades de mundo acima da origem do avatar, do ponto que se apoia na superfície: o quadril para idle/walk, a base das nádegas para `sit`, 0 para `lay`) e `tiles_per_sec` (só no walk; velocidade nativa do clip).

### Pipeline (`tools/asset_pipeline/`)
1. `build_animations.py` (Python puro) lê o pacote, copia `idle`/`walk` e escreve o glTF só com o esqueleto e as animações. O Blender é usado apenas por `sample_mixamo.py` para amostrar a pose do FBX.
2. `rig_math.py` faz o retarget: rotação de mundo de cada osso em relação ao rest pose, mais uma correção que alinha a direção do osso em repouso (A-pose do pacote contra T-pose do Mixamo). O quadril mantém só o deslocamento vertical, escalado pela altura do quadril.
3. `contact_height` e o deslocamento do corpo deitado são medidos deformando a malha do corpo (skinning em Python).
4. `anim_config.py`, `rig_math.py` e `glb_tools.py` são testados por `test_*.py`.
5. `validate_animations.py` garante que os clips existem e que os nós animados têm o mesmo caminho de nomes em todo corpo e peça.
6. Opcional: `build_animations.py --preview-glb` e `render_clip_preview.py` renderizam os clips no Blender para inspeção visual.

### Cliente (`crates/game/src/avatar_anim.rs`, `AvatarAnimationPlugin`)
- `AvatarAnimationLibrary`: carrega o GLB, monta o `AnimationGraph` a partir de `named_animations` e guarda a metadata de `clips.json`.
- `bind_body_skeletons`: no nó `Skeleton` do corpo (`AvatarBodyPart`) insere `AnimationPlayer`, `AnimationGraphHandle`, `AnimationTransitions` e `AvatarAnimator`. Em cada descendente insere `AnimationTargetId::from_names(caminho a partir de Skeleton)` + `AnimatedBy(rig)`.
- `rebind_part_joints`: em cada peça (`AvatarClothingPart`), troca `SkinnedMesh::joints` pelos joints do corpo de mesmo nome.
- `update_avatar_pose`: `PathFollower` + `AvatarPosture` → `AvatarPose`. Começar a andar volta a postura para `Stand`.
- `drive_animators`: transição de 200 ms entre clips. A velocidade do walk é `tiles_per_sec do caminho ÷ tiles_per_sec do clip`, limitada a [0,25; 3].
- `apply_posture_offset`: desloca o `AvatarVisualRoot` em Y por `support_height − contact_height`.
- `AvatarVisualRoot` é girado 180° em Y, porque os modelos olham para +Z e o `look_at` aponta −Z.
- `LAY_CENTER_OFFSET` (`avatar.rs`) é o deslocamento do meio do corpo deitado; o valor sai do log do `build_animations.py`.

## Mudanças de Protocolo / Contratos
Nenhuma nesta etapa. A fase 4 adicionou `posture`/`rotation`/`y` ao `AvatarSnapshot`/`AvatarJoined` e o evento `AvatarPostureChanged` (opcode 113) — ver spec do servidor 0004.

## Critérios de Aceite (Testáveis)
- [x] `character_anims.glb` contém os 4 clips e todos os canais apontam para caminhos existentes nos corpos e peças (`validate_animations.py`).
- [x] `walk` é in-place e traz `tiles_per_sec` em `clips.json`.
- [x] Funções puras testadas: `desired_pose`, `clip_name`, `walk_playback_speed`, `posture_offset`, `remap_joints`, caminhos de alvo e busca da origem da pose.
- [ ] Na sala, o avatar fica em idle parado, anda com o ciclo de caminhada e as roupas acompanham o corpo.
- [ ] No criador de personagem, o preview toca o idle.
- [ ] `P` (build debug, sem campo de texto em foco) alterna Sit → Lay → Stand no avatar local.

## Plano de Testes e Validação
- `python3 -m unittest discover -s tools/asset_pipeline`
- `python3 tools/asset_pipeline/validate_animations.py`
- `cargo test -p game` (módulo `avatar_anim::tests`)
- Teste visual com a stack local (skill `run-local-stack`).
