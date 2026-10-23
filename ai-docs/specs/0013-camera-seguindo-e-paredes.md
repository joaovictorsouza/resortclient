# Spec 0013: Câmera que segue o avatar, zoom e paredes do fundo

## Status
Implemented

## Objetivo
Sair da vista isométrica "Habbo": a câmera fica mais perto do avatar, acompanha o personagem enquanto ele anda pela sala e o jogador ajusta o zoom. A sala ganha paredes só nos dois lados do fundo (vista em corte, como The Sims), sem paredes na frente para não tapar a câmera.

## Câmera
- Perspectiva (FOV 45°) com ângulo fixo de 45° acima do piso e 45° em volta (sem rotação), olhando do canto da frente da sala; as duas paredes do fundo formam um "V" atrás do avatar. O recurso `CameraRig` guarda o alvo e a distância.
- O alvo segue o avatar local com suavização exponencial (independente de FPS) e fica preso aos limites da sala, para a câmera nunca olhar o vazio. Sem avatar local, olha o centro da sala. Ao trocar de sala, o alvo pula para o avatar e o zoom volta ao padrão (9 tiles).
- Zoom: mínimo 4 tiles; máximo = distância que enquadra a sala inteira (no mínimo 12).
- **Celular**: pinch com dois dedos. **Desktop**: rolagem do mouse (`MouseWheel`).
- A rolagem é ignorada quando um dedo está na tela (o `touch_scroll` converte arrasto de um dedo em `MouseWheel` para os modais) e quando o ponteiro está sobre a UI (modais rolam com ela).

## Toque x movimento
- No toque, o comando de mover agora dispara ao **soltar** o dedo, e só se o dedo andou menos de 12 px e a sequência nunca teve dois dedos. Assim pinch e arrasto não movem o avatar. O clique do mouse continua no pressionar.

## Paredes
- `room.rs` gera uma peça por tile de piso cujo vizinho "atrás" (z-1 ou x-1) não é piso: só as bordas voltadas para longe da câmera, também em salas irregulares (octógono, L), onde formam degraus. Altura única de 2 tiles, espessura 0,16, centradas na borda do piso.
- Cada vértice onde duas peças se encontram (ou uma termina) recebe um poste, então cantos e degraus ficam fechados.
- Parede entre a câmera e o avatar local fica translúcida (segmento câmera-avatar cruza a caixa da parede), para nunca tapar o personagem.
- Reconstruídas a cada snapshot de sala (`RoomWall`).

## Fora de escopo
- Girar a câmera, arrastar para mover a vista, paredes que ficam transparentes, porta/janela nas paredes.

## Critérios de Aceite
- [x] Câmera segue o avatar local suavemente e fica dentro da sala.
- [x] Pinch (celular) e rolagem (desktop) com limites mínimo e máximo.
- [x] Pinch/arrasto no chão não enviam `MOVE_REQUEST`.
- [x] Paredes só no fundo, cantos fechados, sem paredes na frente.
- [x] Parede na frente do avatar fica transparente.
