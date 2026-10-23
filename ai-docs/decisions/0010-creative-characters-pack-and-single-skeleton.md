# ADR 0010: Pacote Creative Characters, esqueleto único de 44 joints e poses via Mixamo

## Status
Aceita. Substitui a [ADR 0008](./0008-avatar-animation-pipeline.md) e a parte de pipeline de assets e catálogo da [ADR 0007](./0007-modular-character-pipeline-and-customization.md). A composição modular por subcenas glTF da ADR 0007 continua valendo.

## Contexto
Os personagens vinham do pacote *Teenz 3D Character Collection*, montado sobre o rig Rigify (`rig_kid`, 80 ossos de deformação) dentro de um `.blend` de 1 GB. Isso exigia Blender, scripts de exportação por peça, correção de joints órfãos, remoção de shape keys e um retarget Mixamo → Rigify por frame. As peças eram separadas por gênero e pesavam 144 MB no total.

O jogo passou a usar o pacote *Creative Characters* (versão gratuita para teste; o pacote completo da mesma empresa será comprado). O pacote já vem em GLB, com todas as peças exportadas sobre o mesmo esqueleto de 44 joints (nomes no estilo Mixamo, sem o prefixo `mixamorig:`), um único corpo e 30 animações.

## Decisão

### 1. Um corpo, sem gênero
`AvatarAppearance` perde o campo `gender` e ganha `face_id`. O corpo (`characters/base/body_010.glb`) é fixo; o rosto passa a ser uma peça escolhida (`face`), porque o corpo do pacote não tem olhos nem boca. O protocolo antigo continua legível: o `gender` é ignorado ao desserializar e o `face_id` ausente cai no padrão.

### 2. Importação por ferramenta, sem Blender para as peças
`tools/asset_pipeline/import_character_pack.py` (Python puro, sem dependências):
- classifica cada GLB por prefixo do nome (`Hat_` → `accessory`, `Pants_` → `bottom`...); um prefixo desconhecido é erro, para forçar a classificação consciente de pacotes novos;
- troca o atlas de textura embutido em cada GLB por uma referência a um único `characters/palette.png` (a textura é uma paleta de cores, então é carregada e enviada à GPU uma vez só);
- gera `catalog.json` para o cliente e a cópia idêntica que o servidor embute (`resort-game-server/data/cosmetics/catalog.json`);
- apaga `base/` e `parts/` antes de escrever, então os assets de um pacote anterior desaparecem.

O resultado cai de 144 MB para 2,4 MB.

### 3. Cor por peça: cor sólida em vez de multiplicar a textura
Cada malha do pacote usa um ponto minúsculo da paleta (UV quase constante). Multiplicar `base_color` pela textura misturaria a cor escolhida com a da paleta. O cliente, então, pinta a peça com a cor escolhida e descarta a textura (`paint_flat`). Isso vale só para as peças com `"tint": true` no catálogo: corpo (pele), cabelo, roupas e calçados cuja malha ocupa uma faixa estreita da paleta (`MAX_TINT_UV_SPAN`). Rostos, acessórios e peças multicoloridas (casacos com estampa, fantasias) mantêm as cores pintadas.

### 4. Animações: do pacote, mais poses do Mixamo
`tools/asset_pipeline/build_animations.py` gera `characters/animations/character_anims.glb` e `clips.json`:
- `idle` e `walk` são copiados das animações do pacote. O `walk` perde o movimento para a frente do `Root` (o cliente move o avatar pelo caminho) e o `tiles_per_sec` sai da distância percorrida.
- `sit` e `lay` são um frame final de uma animação do Mixamo (`Sitting.fbx`, `Lying Down.fbx`) retargetado para o esqueleto do pacote. O Blender só é usado para amostrar o FBX (`sample_mixamo.py`); a matemática (`rig_math.py`) é Python puro e testada.
- O retarget transfere a rotação de mundo de cada osso em relação ao rest pose, com uma correção de direção do osso em repouso, porque o pacote repousa em A-pose e o Mixamo em T-pose. O quadril mantém só o deslocamento vertical, escalado pela altura do quadril. O pacote tem 2 juntas de coluna contra 3 do Mixamo: `Spine` segue `Spine1` e `Spine1` segue `Spine2`.
- `contact_height` (onde o corpo toca o assento) é medido deformando a malha do corpo com skinning em Python, após verificar que o rest pose é reproduzido.
- Todo joint é animado em todo clip (as poses mantêm o rest nos ossos que não mudam), para a mistura entre clips nunca herdar valores do clip anterior.

### 5. Esqueleto mestre por avatar (mantido da ADR 0008)
O `AnimationPlayer` fica no nó `Skeleton` do corpo. O caminho de nomes usado pelo Bevy para os alvos de animação parte desse nó (`Skeleton/Root/Hips/...`), igual no corpo, nas peças e na biblioteca de animações; `validate_animations.py` e o teste `rig_root_name_matches_the_assets` protegem esse contrato. As peças têm os joints remapeados, por nome, para os do corpo.

### 6. Posturas reduzidas a Sit e Lay
Os clips `sit_floor` e o `lay` antigo (sofá) deixam de existir: só há `Sit` e `Lay` (deitado de costas, para camas). Voltam quando houver animações para eles.

### 7. Migração de aparências salvas
- O servidor sanitiza a aparência ao carregá-la do Storage (`Catalog.Sanitize`): itens que saíram do catálogo viram o item padrão, cores válidas são mantidas.
- O cliente mapeia peças desconhecidas para o primeiro item da categoria (`resolve_part_id`, `sanitize_appearance`), inclusive antes de reenviar o perfil local ao conectar.

## Alternativas Consideradas
- **Manter o Teenz e só adicionar as poses**: o pacote não tem animação de caminhada e o pipeline Blender/Rigify é caro de manter.
- **Exportar as animações pelo Blender (importar/exportar glTF)**: o exportador reescreve a rotação de repouso de cada osso; os valores absolutos de rotação dos clips deixariam de bater com os nós das peças. Escrever o glTF direto preserva os nós originais.
- **Multiplicar a cor pela textura da paleta**: a cor escolhida se misturaria com a da paleta e o resultado dependeria da peça.
- **Um shader próprio para trocar a região da paleta**: mais flexível, mas aumenta o risco no Android (Adreno), onde o skinning já exigiu um workaround (ver [a investigação](../process/investigacao-android-skinning.md)).

## Consequências
- **Positivas**:
  - Importar um pacote novo é um comando, sem Blender, e a conformidade do rig é validada.
  - Menos draw calls e texturas: uma paleta compartilhada.
  - Menos ossos por avatar (44 contra 80), o que também alivia o skinning no mobile.
- **Negativas / Trade-offs**:
  - Cor sólida perde o gradiente pintado na paleta; a iluminação PBR compensa em parte.
  - Peças multicoloridas não aceitam cor do jogador.
  - Os clips de sentar e deitar são uma única pose; sem transição animada.
  - O pacote gratuito tem 30 peças; o catálogo e a variedade só crescem com o pacote completo.
  - Os avatares salvos com o Teenz perdem as peças e ficam com as peças padrão (as cores são mantidas).
