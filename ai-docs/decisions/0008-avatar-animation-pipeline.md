# ADR 0008: Pipeline de Animação do Avatar (esqueleto único + retarget do Mixamo)

## Status
Substituída pela [ADR 0010](./0010-creative-characters-pack-and-single-skeleton.md) (novo pacote e esqueleto; o esqueleto mestre por avatar continua).

Histórico: Aceita

## Contexto
O avatar é composto por várias cenas glTF (corpo, cabelo, top, calça, sapatos, acessórios), e cada GLB traz sua própria cópia do esqueleto `rig_kid` (ADR 0007). O pacote Teenz tem 51 poses estáticas, mas nenhum ciclo de caminhada. Os ciclos de caminhada disponíveis vêm do Mixamo (`Walking_Boy.fbx`, `Wolking_Girl.fbx`), feitos para o esqueleto `mixamorig`. Era preciso animar o avatar inteiro sem quebrar as 115 peças cosméticas já exportadas.

## Decisão
1. **O rig não muda.** As animações do Mixamo passam por retarget para os controles Rigify do `rig_kid` dentro do Blender (`tools/asset_pipeline/retarget_mixamo.py`). A transferência é feita por rotação em espaço de mundo relativa ao rest pose, o quadril perde o root motion horizontal e o resultado é assado nos ossos DEF pelo exportador glTF (`export_force_sampling`).
2. **Biblioteca de clips separada.** Todos os clips ficam num GLB só de rig (`characters/animations/kid_anims.glb`), compartilhado por meninos e meninas, com metadata em `clips.json`.
3. **Um esqueleto mestre por avatar.** O `AnimationPlayer` fica no nó `rig_kid` do corpo, e os alvos de animação são gerados no cliente com a mesma convenção do loader do Bevy (caminho de nomes a partir do `rig_kid`). As peças cosméticas têm `SkinnedMesh::joints` remapeado, por nome, para os joints do corpo. As inverse bindposes são idênticas porque todas as peças são exportadas do mesmo rig.

## Alternativas Consideradas
- **Auto-rig do Mixamo (ou do Meshy) no corpo**: as roupas e cabelos teriam de ser re-skinados com transferência de pesos, com risco alto de interpenetração e revisão peça por peça.
- **Renomear o rig para a nomenclatura do Mixamo**: as animações do Mixamo funcionariam direto, mas seria preciso fundir os ossos de torção (`*.001`) nos grupos de vértices de todas as malhas, piorando a deformação de cotovelos e ombros.
- **Um `AnimationPlayer` por peça**: não exige remapear joints, mas são N players por avatar, com risco de dessincronia e custo de CPU proporcional ao número de peças.
- **Animação procedural no Bevy**: sem pipeline, porém visual pobre e sem reaproveitar as poses do pacote.

## Consequências
- **Impactos Positivos**:
  - Roupas, cabelos e acessórios seguem o corpo sem reexportar nenhuma peça.
  - Qualquer animação nova do Mixamo entra adicionando uma linha em `MIXAMO_CLIPS` e rodando o export.
  - Há um único player por avatar, então trocar a aparência só re-vincula os joints.
- **Impactos Negativos / Trade-offs**:
  - As proporções chibi diferem das do Mixamo, e alguns movimentos podem precisar de offsets por osso (`RETARGET_OFFSETS_DEG`).
  - Os ossos do esqueleto próprio de cada peça continuam existindo como entidades ociosas.
  - O caminho de nomes do `rig_kid` precisa ser igual entre corpo e biblioteca; `validate_animations.py` protege esse contrato.
