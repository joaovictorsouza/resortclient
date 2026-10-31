# Animais (crate `animals`, pets do servidor)

Decisões: [ADR 0016](../decisions/0016-crate-de-animais.md). Servidor: spec 0008 e `contract/protocol.md`, seção 19.

## Adicionar um cachorro (nova raça)
1. No Blender, siga o padrão de nomes do `Dog_ShihTzu.blend`: armature `Dog_<Raça>_rig` (mesmos ossos), malha `Dog_<Raça>`, `_MouthInterior`, `_Teeth`, `_Tongue`, ações `idle walk run bark pant_tongue_out`.
2. Exporte (uma vez por espécie sem `--meshes-only`; as demais raças só as malhas):
```bash
Blender -b --factory-startup --python tools/animals/export_animal.py -- Dog_<Raça>.blend assets/animals dog <raça> [--meshes-only]
Blender -b --factory-startup --python tools/animals/export_animal.py -- --thumbnail Dog_<Raça>.blend assets/ui/thumbs/pet_dog_<raça>.png
```
3. No servidor: raça e pelagem em `data/pets/catalog.json`, item `category: "pet"` em `data/furniture/catalog.json` e oferta em `data/shop/catalog.json`; `go test ./internal/items -run TestExportedCatalogIsUpToDate -update-catalog` e `scripts/sync-contract.sh` no cliente.
4. Publicar: `resort-assets` (`scripts/publish-assets.sh dev`), depois o servidor.

## Ver sem servidor
```bash
cargo run -p game --example pet_preview -- saida   # um "servidor falso" manda as mensagens; ROOM_ASSETS, PET_CAM=side, TOON_AVATAR_TIER
```
`GAME_PET=off` esconde os pets no jogo.
