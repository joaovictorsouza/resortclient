# Spec 0005: Pipeline de Assets de Personagem e Criador de Personagem

## Status
Approved. Revisada em 2026-10-21 para o pacote *Creative Characters* ([ADR 0010](../decisions/0010-creative-characters-pack-and-single-skeleton.md)); o texto original descrevia o pacote Teenz.

## Contexto e Objetivo
Permitir que usuários personalizem seus avatares 3D com a coleção modular do pacote de personagens: rosto, tom de pele, cabelo, roupas (superior e inferior), calçados e acessórios. O sistema contempla a importação automatizada dos assets glTF, a validação autoritativa no servidor Nakama (Go), a persistência local e uma interface 3D no cliente Bevy com estúdio, rotação interativa e paletas de cores.

## Escopo
- **Dentro do escopo**:
  - Importação por `tools/asset_pipeline/import_character_pack.py` (Python puro): classifica as peças por prefixo do nome, compartilha a paleta de textura (`characters/palette.png`) e gera `catalog.json` no cliente e no servidor. Guia: [importar-pacote-de-personagens.md](../process/importar-pacote-de-personagens.md).
  - Catálogo canônico em `assets/characters/catalog.json` e `resort-game-server/data/cosmetics/catalog.json` (idênticos). Campos por item: `id`, `name`, `category` (`body`, `face`, `hair`, `top`, `bottom`, `shoes`, `accessory`), `asset_path` e `tint`.
  - `AvatarAppearance` (opcodes 7 `UpdateAppearanceRequest` e 112 `AvatarAppearanceUpdated`): `skin_color`, `face_id`, `hair_id`/`hair_color`, `top_id`/`top_color`, `bottom_id`/`bottom_color`, `shoes_id`/`shoes_color`, `accessories`. Não há gênero: o corpo é único.
  - Armazenamento no Nakama Storage (`user_profiles/avatar_appearance`) e RPCs `save_avatar_appearance`/`get_avatar_appearance`. Aparências salvas com itens que não existem mais são sanitizadas ao carregar.
  - Renderização modular nas salas e no estúdio: corpo + uma subcena glTF por peça, `Collider::capsule(0.3, 1.2)` e `RigidBody::Kinematic`.
  - Cor por peça: peças com `tint: true` são pintadas com a cor escolhida (cor sólida, sem a textura); as demais mantêm as cores do modelo.
  - Tela do Character Creator (`AppState::CharacterCreator`): estúdio 3D, rotação (botões e arrasto), aleatorização, abas Rosto, Pele, Cabelo, Superior, Calças, Calçados e Acessórios, paletas de cor e botão "Confirmar e entrar no jogo".
- **FORA do escopo**:
  - Blend shapes faciais (o rosto é uma peça trocável, não uma expressão animada).
  - Escolha de corpo. Microtransações de cosméticos.

## Componentes Bevy Envolvidos

### Recursos (`Resource`)
- `CharacterCreatorState`: aparência ativa, categoria selecionada, ângulo de rotação, flag de arrasto e retorno à sala.
- `CharacterCatalog`: catálogo embutido em tempo de compilação (`catalog.json`).

### Componentes (`Component`)
- `CharacterCreatorRoot`: entidades da cena de criação, para limpeza na saída.
- `AvatarPreviewRoot`: raiz do modelo 3D em preview.
- `AvatarPartTint`: cor sólida aplicada às malhas de uma peça (`paint_flat`).

## Critérios de Aceite
- [x] Todas as peças do pacote importadas para `assets/characters/` e listadas em `catalog.json` (cliente e servidor).
- [x] Opcodes 7 e 112 codificam e decodificam `AvatarAppearance` com `face_id` e sem `gender`; JSON antigo ainda é lido.
- [x] O servidor valida categorias, IDs existentes e cores hex antes de persistir, e sanitiza aparências antigas ao carregar.
- [x] Jogadores conectados ou que alteram a aparência têm o avatar renderizado com as peças certas.
- [x] Trocar peças ou cores no criador atualiza o avatar 3D na hora; avatares salvos com peças que não existem mais recebem o item padrão da categoria.
- [x] "Confirmar e entrar no jogo" salva a aparência no perfil local e envia ao servidor.
- [x] Testes: `cargo test`, `go test -race ./...` e `python3 -m unittest discover -s tools/asset_pipeline`.
