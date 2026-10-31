# Pet decorativo no lobby (Mochi, shih-tzu)

NPC só do cliente: não há mensagem de rede, colisão com jogadores nem persistência. Vive em `crates/game/src/pet.rs` e aparece apenas na sala `lobby`.

## Comportamento
Máquina de ações aleatórias com pesos (`choose_action`): parado, sentar, deitar, passear (clipe `walk` acelerado), trotar (clipe `run`), latir e ofegar (língua para fora). Quem descansa (sentado ou deitado) volta a ficar de pé antes de andar. As caminhadas são buscas em largura (4 vizinhos) sobre os tiles de chão sem móvel sólido (`free_tiles`); móveis colocados depois da entrada não são considerados.

## Estilo, LOD e aparelho
O modelo é glTF com materiais padrão, então o `SceneStyle` do `toon_avatar` aplica o toon de sala/móveis sozinho. Três níveis (`Quality`): `lod0` ≈ 9,5 mil triângulos, textura 1024; `lod1` ≈ 3,4 mil, 512; `lod2` ≈ 0,9 mil, 256 (sem boca/dentes/língua). O nível vem do tamanho na tela (limiares próprios do pet, 140 e 55 px) e do `DeviceTier`, como nos avatares.

## Assets (`resort-assets`, camada lazy, TTL 30 dias)
`animals/mochi/{lod0,lod1,lod2}.glb`, `anims.glb` (armature + ações `idle walk run bark pant sit lay`) e `clips.json` (duração, loop, velocidade em m/s no campo `tiles_per_sec`, `model_height_m`). Gerados por `tools/animals/export_mochi.py` (Blender) a partir do `.blend` do Mochi; `sit` e `lay` são poses procedurais com respiração. Sem os arquivos no manifest o pet fica desligado (aviso no log).

## Ver sem servidor
```bash
cargo run -p game --example pet_preview -- saida   # ROOM_ASSETS=../resort-assets/assets, PET_CAM=side, TOON_AVATAR_TIER=low|mid|high
```
`GAME_PET=off` desliga o pet no jogo.
