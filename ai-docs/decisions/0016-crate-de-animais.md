# ADR 0016: Animais numa crate reutilizável (`animals`) e pets autoritativos no servidor

## Status
Aceita (branch `feat/mochi-pet`; falta validar em aparelho). Substitui o pet decorativo só do cliente da primeira versão.

## Contexto
O dono quer animais que o jogador compra na loja, guarda no inventário e coloca na sala, onde se comportam sozinhos. O primeiro é o shih-tzu; outros cachorros seguem o mesmo padrão e a lógica deve servir a uma crate interna no futuro. O comportamento é decidido pelo servidor, porque o animal é um NPC comprável.

## Decisão
1. **Crate `crates/animals`** (sem dependência de `game`, `protocol` ou rede; usa o LOD e o estilo toon de `toon_avatar`). Ela só desenha e anima: o jogo muda `AnimalAction`, `AnimalRoute` (posições do mundo e velocidade) e `AnimalFacing` numa entidade com `Animal { species, breed, coat, height }`. O jogo descreve cada espécie em `AnimalConfig` (lido de `clips.json`); a crate não lê JSON.
2. **Espécie, raça e pelagem.** Raças da mesma espécie compartilham esqueleto e clipes: um `anims.glb` e um `clips.json` por espécie, malhas por raça. Layout na CDN (camada lazy, TTL 30 dias): `animals/<espécie>/{anims.glb,clips.json}` e `animals/<espécie>/<raça>/lod{0,1,2}.glb`. Pelagem é textura: hoje só `default` (a do modelo); outras trocarão a textura base da raça.
3. **Servidor autoritativo** (`resort-server`, spec 0008, protocolo 3.1): o pet é um item de catálogo `category: "pet"` com `pet {species, breed, coat}`; a loja e o inventário não mudam. Colocado na sala vira uma entidade com comportamento sorteado por pesos (dados em `data/pets/catalog.json`), caminhadas por A*, no máximo 3 por sala, sem bloquear tiles. O cliente espelha `ROOM_SNAPSHOT.pets` e as mensagens `PET_*` (`crates/game/src/pet.rs`).
4. **Interações preparadas.** `PetInteractionRequest` (mensagem do jogo) vira `PET_INTERACT_REQUEST`; o servidor roteia por `kind` até um handler registrado e responde `PET_INTERACT_RESULT` (`NetPetInteractResultEvent`). Nenhuma interação existe ainda; ferramentas futuras só registram handlers e escutam o resultado.
5. **Pipeline do Blender** (`resort-assets/tools/animals/export_animal.py`): o `.blend` segue um padrão de nomes (`Dog_<Raça>`, `Dog_<Raça>_rig`, ações sem prefixo) que serve a qualquer cachorro. `sit` e `lay` são poses procedurais com respiração; a miniatura do inventário sai do mesmo script.

## Consequências
- Positivas: novos cachorros são só um `.blend` + uma entrada de catálogo; o cliente não decide nada do comportamento; a crate é separável.
- Negativas: o pet só aparece com o servidor novo e os arquivos na CDN; `lay` é uma pose simples; a velocidade de passeio vem de uma medição aproximada dos pés (`clips.json`).
- Sem compatibilidade com o pet local da primeira versão: ele foi removido (o app não está publicado).
