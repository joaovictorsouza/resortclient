# ADR 0007: Pipeline Modular de Personagens Teenz 3D e Customização em Camadas

## Status
Parcialmente substituída pela [ADR 0010](./0010-creative-characters-pack-and-single-skeleton.md): o pacote Teenz, o pipeline Blender e a separação por gênero foram trocados; a composição modular por subcenas glTF continua valendo.

Histórico: Aceito

## Contexto
O jogo necessitava de um sistema completo de avatares com alta qualidade visual (estilo Habbo 3D), suporte a customização modular (gênero, pele, cabelo, roupas superiores e inferiores, calçados e acessórios) e compatibilidade com o ecossistema Bevy 0.19, Avian 3D 0.7 e Nakama 3.41. O pacote comercial *Teenz 3D Character Collection* fornece centenas de modelos e peças montados sobre o rig Rigify no Blender (`Teenz.blend`), necessitando de um pipeline automatizado de exportação para glTF sem depender de conversões manuais e propensas a erro.

## Decisão

### 1. Extração Headless e Otimizada via Blender Python
- Criamos a ferramenta `tools/asset_pipeline/export_teenz.py` invocada via CLI do Blender (`Blender -b "Temp Package/Teenz.blend" --python tools/asset_pipeline/export_teenz.py`).
- O script analisa coleções e malhas ativas, desvincula dados temporários, converte curvas para malhas quando necessário (`hair_basement`), aplica deformadores preservando o esqueleto e exporta peças individuais com `export_def_bones=True`.
- Isso reduz o rig Rigify de centenas de ossos controladores para os 80 ossos de deformação reais (`rig_kid`), tornando o carregamento e renderização leves no Bevy.

### 2. Composição Modular por Subcenas glTF
- Em vez de fundir malhas em um modelo único ou tentar costurar texturas em tempo de execução:
  - O avatar é uma entidade raiz com o corpo base (`body_boy.glb` ou `body_girl.glb`) e colisor físico Avian (`Collider::capsule(0.3, 1.2)`).
  - Cada peça cosmética (`hair`, `top`, `bottom`, `shoes`, `accessories`) é instanciada como uma entidade filha carregando sua subcena glTF (`WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0)...))`).
  - O esqueleto base e as peças compartilham a mesma hierarquia de joints, permitindo poses e animações sincronizadas.

### 3. Sincronização Autoritativa no Nakama
- O protocolo define a mensagem compacta `AvatarAppearance` com campos serializáveis em JSON.
- O servidor valida o gênero, a existência dos IDs de peças no catálogo canônico e a formatação das cores hexadecimais antes de autorizar a atualização.
- Os dados são persistidos no Nakama Storage (`user_profiles/avatar_appearance`) e incluídos nos snapshots das salas (`RoomSnapshot`) e eventos de entrada (`AvatarJoined`), permitindo que outros jogadores vejam imediatamente o avatar correto.

### 4. Interface Estúdio 3D Reativa no Bevy
- O criador opera sob o estado dedicado `AppState::CharacterCreator`, isolado da simulação de salas.
- Um estúdio 3D com iluminação tridimensional, câmera orbital e pedestal renderiza o avatar em tempo real.
- A interface exibe abas temáticas, seletores visuais de paleta de cores e cards com o estado ativo destacado.
- As mudanças são refletidas instantaneamente na cena 3D através de re-instanciação reativa quando o recurso `CharacterCreatorState` sofre mutação.

## Consequências

### Positivas
- 115 componentes modulares gerados e catalogados com zero esforço manual após o script.
- Carregamento assíncrono padrão do Bevy com reutilização de shaders PBR.
- Flexibilidade total para adicionar novas peças e coleções futuras apenas incluindo os arquivos e executando o pipeline.
- Sincronização em tempo real entre todos os jogadores nas salas.

### Negativas / Mitigações
- Múltiplas subcenas glTF geram draw calls adicionais para cada avatar (mitigado pelo Bevy auto-batching e LODs futuros para ambientes com dezenas de jogadores simultâneos).
