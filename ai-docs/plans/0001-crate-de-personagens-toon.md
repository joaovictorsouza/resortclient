# Plano 0001: personagens toon do jogo a partir do `anime-character-bevy`, empacotados como crate reutilizável

Status: **implementado nas fases F0 a F7 em 30/10/2026 (ver [ADR 0015](../decisions/0015-crate-de-personagens-toon.md)); F8 parcial e F9 futura**; decisões do dono aplicadas em 30/10/2026 (seção 15); cor da pele resolvida por medição (15.13). Substitui o plano anterior sobre o pacote Rukha93 (descartado). Este documento é só planejamento: nenhum código do jogo, do protótipo nem do pacote Rukha93 foi alterado.
Licenças de pacotes de terceiros: ignoradas por ora; a licença limitada dos assets é o motivo de a crate ir, no futuro, para um repositório **privado** próprio. **Decisão de 30/10/2026**: por enquanto a crate vive **no mesmo repositório do jogo** (workspace do `resortclient`) e a separação fica para depois (fase 9, futura); os assets nunca entram nesse repositório.
Contexto: [ADR 0007](../decisions/0007-modular-character-pipeline-and-customization.md), [ADR 0010](../decisions/0010-creative-characters-pack-and-single-skeleton.md), [ADR 0011](../decisions/0011-assets-via-cdn.md), [investigação do skinning no Android](../process/investigacao-android-skinning.md).

## 1. Resumo

- **Objetivo**: transformar o protótipo `~/Projetos/anime-character-bevy` numa **crate Bevy reutilizável** (plugins, sem nada específico do Resort Game), usá-la nos personagens do jogo e manter a tela de montagem de personagem do protótipo só como **demonstrador** (um exemplo da crate).
- **O protótipo está bem mais adiantado do que o plano anterior supunha**: tem material toon com cor por máscara, rim e outline falso, atlas que desenha pele e rosto em **uma** draw call, mesclagem de malhas, peças presas à cabeça, roupas customizadas ajustadas ao rig (`fit_garment.py`) e **LODs próprios** (`lod1`, `lod2`) com tiers de qualidade. O plano reaproveita isso e conserta o que o impede de virar biblioteca.
- **O que o protótipo não tem e o jogo precisa**: animações (nenhuma), expressões e piscada, pegar objetos (copo, lata), qualidade **por avatar** (hoje um tier global para a cena toda), cores **por avatar** (hoje por nome de material, o que vaza entre avatares), ligação com protocolo/servidor/CDN e uma API de crate.
- **Visual cartoon opcional para salas e objetos**: opção de configuração que troca os materiais do cenário por uma variante toon (e volta), medida e testada em captura e em aparelho; os personagens são sempre toon (seção 9).
- **LOD**: o esquema conversado antes (por aparelho + por tamanho do avatar na tela, animação mais lenta nos distantes) **continua válido** e é conciliado com o do protótipo na seção 6: as malhas, o gerador (`make_lods.py`) e as pastas de textura por tamanho do protótipo viram os níveis L0/L1/L2; o que falta é a decisão **por avatar**.
- **Fases**: F0 consolidar e medir o protótipo → F1 extrair a crate → F2 LOD por avatar e tier de aparelho → F3 animações → F4 rosto (olhos, expressões, piscada) → F5 objetos na mão (beber) → F6 integrar ao jogo (protocolo, servidor, CDN, criador) → F7 visual cartoon de salas e objetos → F8 pipeline, console e CDN → F9 (futura) separar a crate num repositório privado no GitLab. Estimativas na seção 13.
- **Decisões do dono (30/10/2026)**, todas na seção 15: crate dentro do repositório do jogo (separação futura); corpos masculino e feminino de volta; até 50 avatares por sala; expressões pelo servidor; cenário cartoon desligado por padrão até o teste; animações do Mixamo, clips fixos; **personalização máxima**; troca total do pacote Creative Characters; cabelo estático. O pacote **não tem calçado masculino próprio** (usa o feminino nos dois corpos; seção 3).

## 2. O que o protótipo já tem (lido em 29/10/2026; **não executei** o protótipo)

Pasta `~/Projetos/anime-character-bevy` (Bevy 0.20, crate `lib` + `cdylib`, **repositório git sem nenhum commit**; ~2,1 mil linhas de Rust em `src/` e scripts Python/Blender em `tools/`).

| Módulo / ferramenta | O que faz | Observação para a crate |
|---|---|---|
| `toon.rs` + `assets/shaders/toon.wesl` | `ToonMaterial` (porte do `ShaderGraph_CharacterToon`): 3 regiões de cor por máscara RGB (A, B, C × 2 tons), rampa de luz, rim, outline falso, normal map por derivadas de tela, `alpha_cutoff`, `Normal Adjust` | cores por nome de material glTF em `ColorOverrides` global: **vaza entre avatares** |
| `atlas.rs` + `toon_atlas.wesl` | pele e as peças do rosto (rosto, olho, boca, cílios, sobrancelha, brilho) mesclados numa malha com UVs de um atlas, id da peça por vértice (cor de vértice), morph targets mesclados e pesos repassados: **1 draw call** (log: "atlas merge: 11 pieces in one draw call") | núcleo do ganho de desempenho |
| `merge.rs` | mescla malhas skinned que partilham material e esqueleto (log: "5 draw calls saved") | refaz a mescla quando o conjunto visível muda |
| `quality.rs` | `Quality` Low/Mid/High: malhas `lod2`/`lod1`/originais, texturas 512/1024/2048, sem normal map e sem outline no Low, sombra 2048/1024/nenhuma, MSAA off/4x/4x; padrão Mid no celular, High no desktop; `PROTO_QUALITY` sobrescreve; `Q` alterna e recarrega | **tier global** (um recurso para a cena toda) e sem detecção real de aparelho |
| `tools/make_lods.py` | Blender: Decimate com bordas protegidas (grupo de vértices `keep`), pesos acompanham os vértices, `--strip-morphs` no nível mais baixo, mantém nomes de nó, ordem de juntas e materiais | gera `characters/lod1` (33 MB de GLB) e `lod2` (5,7 MB) |
| `attach.rs` | cabelo, chapéu e óculos como cenas filhas do osso `CC_Base_Head`, cancelando a escala do esqueleto; chapéu substitui o cabelo | cadeias de ossos do cabelo ficam estáticas |
| `lib.rs` | catálogo F/M, `Look` por índices, corpo + roupas como cenas glTF e reapontamento dos joints por nome ao corpo, `BodyPart` para esconder o corpo sob a roupa, câmera, luz, captura offscreen (`PROTO_SNAPSHOT`) | tudo em `main()`; catálogos por `include_str!` |
| `ui.rs` | barra de botões e painel de cores (paleta por região) | vira o demonstrador |
| `tools/` | `fbx_to_glb.py`, `inventory.py`, `export_materials.py`, `export_outfits.py`, `export_attachments.py`, `fit_garment.py` (ajusta uma roupa modelada em manequim ao avatar, copia os pesos do corpo, 4 influências, decima), `tga_to_png.py`, `check_skeletons.py`, `texconv` (PNG→Basis, UASTC/ETC1S e atlas), `bundle_ios.sh` | pipeline de assets **sem** nada do jogo |
| `build/` | logs de execuções em `Low`/`Mid`/`High`, capturas, pasta `android` | indício de testes locais; não verifiquei resultados em aparelho |

Números que vi nos logs de 29/10 (quantidade de triângulos visíveis com roupa, cabelo e chapéu; variam com as peças escolhidas): **Low ≈ 4–5 mil, Mid ≈ 9–11 mil, High ≈ 12–16 mil**; com o atlas, rosto e pele são 1 draw call. Tamanho em disco: texturas `.basis` 512 px = 23 MB, 1024 px = 82 MB, 2048 px = **222 MB**; GLBs: originais 36 MB, `lod1` 33 MB, `lod2` 5,7 MB (o `lod1` quase não reduz o arquivo: o ganho dele é em triângulos, não em bytes).

**Limitações que a crate precisa resolver**: não há **nenhuma animação** (nem idle) e o pacote Rukha93 também não traz; qualidade e cores são globais; `include_str!`/caminhos fixos (`characters/...`); o código de entrada e a UI estão misturados com a lógica; dependência dos assets locais; sem testes automatizados; sem git.

## 3. O que o pacote Rukha93 fornece (resumo; medido no Blender 5.2 headless)

106 FBX, 143 MB, Unity Built-in, **sem LOD, sem animações**, esqueleto Reallusion `CC_Base` com **71 juntas** (92 nós com `_end`), 14 de torção e dedos. Corpo F/M em 12 submalhas (~6,3 mil triângulos), 4 rostos por corpo (~5 mil triângulos, **46 blend shapes**), 22 cabelos (até 16 influências por vértice), 8 chapéus/variantes, 2 óculos, 3 cachecóis, 12 tops, 6 bottoms, 5 conjuntos, 7 calçados (**só `F_shoes`; não há `M_shoes`**: no sample do Unity os 7 `Shoes.0x.asset` ficam na pasta `Shared`, apontam para as malhas `F_shoes` e escondem a parte `Legs_Feet` (33) do corpo, nos dois corpos; os pés dos corpos F e M têm as mesmas dimensões nas minhas medições, então o encaixe deve servir, **a conferir visualmente na F0**), texturas só de **máscara RGB + normal** (sem albedo; 2048² no corpo, top e conjunto, 1024² no cabelo) e `ColorRamp.png` 512×16. Os 3 shaders toon, os subgrafos e o sample de personalização (`CustomizationDemo.cs`: equipar = reutilizar `bones`/`rootBone` do corpo, ocultar `BodyPartType`, `outfit` remove top e bottom) foram lidos e já estão portados no protótipo.

Blend shapes do rosto (46): ajustes (`eye_size`, `nose_length`, `tuck_ears`, `shorten`, `eyelashes_1`), expressões (`happy_01..03`, `angry_01..03`, `sad_01..03`, `surprised_01`, `extra_01`), olhos (`eye_look_*` ×8, `eyes_closed.01..03`, `eyes_wide`, `eye_iris_small`), sobrancelhas (`eyebrows_up/down`), boca (`jaw_open`) e 10 fonemas. A função exata de cada uma (por exemplo, qual `eyes_closed` é o piscar completo) **precisa ser conferida visualmente na F4**; não assumi.

## 4. O que existe hoje no jogo (resumo)

Cliente `resortclient` (Bevy 0.20, branch `main`): `avatar.rs` e `avatar_anim.rs` (um `AnimationPlayer` no nó `Skeleton`, joints das peças remapeados por nome, 44 juntas, `character_anims.glb` + `clips.json`), `character_creator.rs`, `appearance.rs`, `cdn_assets.rs`/`catalog_sync.rs` (CDN-only com tiers `core`/`lazy`), `gpu_compat.rs` (Adreno 650/730 com skinning por uniform buffer), `room.rs` e `furniture.rs` (materiais `StandardMaterial` para pisos, paredes, ghosts e móveis). Protocolo: `AvatarAppearance` com ids e cores hex por slot e `accessories`; servidor Go (`Catalog.Sanitize`, catálogo embutido, `get_catalogs`). Assets no `resort-game-assets` (privado, CDN R2, `build_manifest.py` com `core`/`lazy` e `ttl_days`, console web). O corpo atual tem 3,2 mil triângulos e 44 juntas.

## 5. Arquitetura da crate

### 5.1 Onde a crate mora (decidido em 30/10/2026: dentro do repositório do jogo)

A crate nasce como membro do workspace do `resortclient`, ao lado de `crates/game`, `crates/protocol` e `crates/nakama_client` (ADR 0003):

```
resortclient/
  crates/toon_avatar/        # a crate reutilizável (nome provisório; features abaixo)
  crates/toon_avatar_demo/   # demonstrador: tela de montagem do ui.rs + cenas de teste (binário/cdylib)
  crates/game/               # passa a depender de toon_avatar (path)
```

- **Sem assets no repositório**: nem o pacote Rukha93, nem GLB, nem textura. Os assets e o pipeline (FBX→GLB, LODs, texturas, atlas, retarget) ficam no repositório privado `resort-game-assets`, como já manda a ADR 0011; o demonstrador lê os assets da CDN (DEV) ou de uma pasta local ignorada pelo git (`TOON_AVATAR_ASSETS=...`).
- **Fronteira para separar depois**: a crate **não pode** depender de `game`, `protocol`, `nakama_client`, `bevy_hui` nem do host KMP; tem `Cargo.toml` próprio, `publish = false`, versão semântica própria, `README` e `CHANGELOG` desde o início, e o jogo a usa por `path`. Um teste de CI (`cargo tree -p toon_avatar` sem nenhuma crate do jogo) protege essa regra; com ela, separar na fase 9 vira mover a pasta com `git subtree split` e trocar a dependência por `git = ..., tag = ...` (o mesmo padrão de `bevy-kmp-ffi`).
- **Nomes** (`toon_avatar`, `toon_avatar_demo`) são provisórios (decisão 15.2).
- Versão do Bevy **fixada** à do jogo (0.20); a crate declara `bevy` sem features de plataforma (as features `basis-universal` e `android-native-activity` ficam no app).
- Features: `demo-ui` (bevy_ui do demonstrador, desligada por padrão), `basis` (texturas `.basis`), `debug-capture` (captura offscreen para testes visuais).
- Repositório do jogo é público (ADR 0011/0012): o **código** do porte dos shaders e da crate ficará visível enquanto a crate morar aqui; a restrição de licença dos assets não é violada porque nenhum asset entra, mas o porte dos shaders do pacote é um ponto de licença a rever antes de tornar o repositório público (licenças estão ignoradas por ora, a pedido).

### 5.2 API pública (sem nada do Resort Game)

Plugins independentes e combináveis (`add_plugins(ToonAvatarPlugins)` ou um a um):

| Plugin | Responsabilidade | Origem no protótipo |
|---|---|---|
| `ToonMaterialPlugin` | `ToonMaterial`/`AtlasMaterial`, shaders embutidos no binário (`embedded_asset!`), rampa carregada como dado | `toon.rs`, `atlas.rs` |
| `AvatarPlugin` | spawn, `AvatarLook` (dados puros, serde), equipar/trocar peças, ocultar partes do corpo, cabelo/chapéu/óculos, mesclagem e atlas | `lib.rs`, `attach.rs`, `merge.rs` |
| `AvatarLodPlugin` | `DeviceTier`, `QualityProfile`, LOD por avatar, taxa de animação | `quality.rs` + novo |
| `AvatarAnimationPlugin` | biblioteca de clips, estados (idle/walk/sit/lay/beber), camadas e máscara de corpo superior, taxa reduzida | novo (substitui o `avatar_anim.rs` do jogo) |
| `AvatarFacePlugin` | piscada, expressões, direção do olhar, ajustes do rosto (morphs) | novo |
| `AvatarPropsPlugin` | sockets nas mãos, objetos segurados, poses de pegada | novo |
| `ToonStylePlugin` | estilo cartoon opcional para o **cenário** (seção 9); usa a mesma rampa | novo |

Princípios de API:
- **Entrada**: `Avatar { look: AvatarLook, lod_policy }` como componente; mudar `AvatarLook` (um `Component` com `Changed`) reconcilia as peças **sem recriar** o avatar (o criador do jogo já faz isso hoje). Comandos/mensagens: `SetExpression`, `PlayClip`, `Hold { prop }`, `Release`.
- **Saída**: `AvatarReady`, `AvatarLodChanged`, estado de carregamento por avatar.
- **Assets**: a crate **não** assume a origem; recebe um `AvatarAssetRoot` (prefixo de caminhos) e carrega tudo pelo `AssetServer` do app, de modo que o jogo reaproveita o `CdnReader` e seu comportamento de espera/lazy. Catálogo e `materials` entram como **assets JSON** (não `include_str!`).
- **Sem estado global por nome de material**: cores e parâmetros ficam **no material de cada peça de cada avatar** (ou num buffer por avatar indexado por `MeshTag`), corrigindo o vazamento do protótipo (teste automatizado com dois avatares).
- **Sem dependências do jogo**: nem Nakama, nem protocolo, nem `bevy_hui`. O jogo faz a ponte `AvatarAppearance` → `AvatarLook`.
- Shaders e rampa: o código WGSL/WESL é **embutido** na crate (assets de dados, não código, é que vêm da CDN: regra da ADR 0011).

### 5.3 Demonstrador

A tela de montagem atual (`ui.rs`: botões por categoria, painel de cores por região, toggles de outline/merge/qualidade) vira um exemplo/binário da crate de demonstração, usado para: validar visualmente cada versão, gerar capturas de regressão, e mostrar a crate em outros jogos. **O criador de personagem do Resort Game não usa esse código**; usa a API da crate com a UI própria (`bevy_hui`).

## 6. LOD: conciliando o esquema do protótipo com o que foi conversado

**O que o protótipo oferece**: três níveis de **malha** (originais, `lod1`, `lod2`) gerados por Decimate com bordas protegidas e mesma ordem de juntas, três tamanhos de **textura** (2048/1024/512), recursos por tier (normal map, outline, sombra, MSAA), padrão por plataforma (celular = Mid), alternância manual. **O que falta**: detectar o aparelho de verdade, decidir **por avatar** (hoje é um `Resource` global; trocar de tier recarrega a cena inteira), reduzir a taxa de animação dos distantes, e limitar o que se baixa da CDN.

**O que ficou combinado antes (continua válido)**: aparelhos melhores usam alta ou média no jogador local e nos próximos; **baixa nos muito distantes**, com **animação em frequência menor**; detecção do aparelho para definir high/mid/low.

**Conciliação**:

| Conceito | Protótipo | Plano |
|---|---|---|
| Níveis de malha e textura | Low = `lod2` ≈ 4–5 mil triângulos, 512 px, sem normal; Mid = `lod1` ≈ 9–11 mil, 1024 px; High = original ≈ 12–16 mil, 2048 px | **adotar esses três como L2/L1/L0** (já existem, já são gerados por `make_lods.py`). A meta antiga de "≤ 3 mil triângulos no distante" vira um **possível L3** a medir na F2 (ver 15.8); só se o L2 do protótipo (4–5 mil) não bastar nos distantes |
| Escopo | global (cena inteira) | **por avatar**, em `AvatarLod`; o tier do aparelho é só o **teto** |
| Decisão do nível | manual (`Q`) ou padrão por plataforma | `min(política por tamanho na tela, teto do tier)`; política abaixo |
| Recursos de cena (sombra, MSAA) | por tier | **continuam por tier de aparelho** (são da câmera/luz, não do avatar); sombra projetada só do jogador local e próximos |
| Animação | não existe | **taxa por nível**: cheia no local e nos próximos; 10 Hz nos distantes; 5 Hz muito longe; sem avaliar fora da tela (seção 7) |
| Rosto | morphs só no original/`lod1`; `lod2` sem morphs | **igual**: piscada e expressões só em L0/L1; L2 sem morphs (no tamanho de tela de L2 não se vê) |
| Juntas | mesmas 71 em todos os níveis | manter a mesma ordem de juntas (as animações valem em todos); **reduzir juntas no L2** (sem dedos e sem torção, ~24) é uma otimização opcional a medir |
| Draw calls | atlas (pele+rosto = 1) + mescla | **mantidos em todos os níveis**; meta ≤ 6 draw calls por avatar (atlas 1, cabelo, top, bottom, shoes, acessório) contra ~24 sem otimizar |
| CDN | tudo local | só baixar os arquivos **do nível permitido** pelo tier do aparelho |

**Detecção do aparelho (`DeviceTier`)**, executada uma vez antes do renderer e reavaliada em execução, no módulo `AvatarLodPlugin` da crate (a **tabela** de aparelhos mora no jogo, que injeta uma função `fn(&DeviceInfo) -> Tier`, porque depende do que o jogo suporta):
1. Sinais: backend e `AdapterInfo` do wgpu, modelo do aparelho (iOS `hw.machine`, Android `Build.MODEL`/SoC via host KMP), RAM, núcleos, sistema, e o filtro Adreno do `gpu_compat.rs` (650/730).
2. Tabela inicial (hipótese, calibrar na F0): **High** = iPhone 15 Pro/16/16 Pro, iPad M, desktop com GPU discreta, Adreno 740+ ou Mali-G715+ com ≥ 8 GB; **Mid** = iPhones A15/A16, Galaxy S22 (Adreno 730 com o workaround), Android intermediário com ≥ 6 GB; **Low** = iPhones antigos, Android com < 6 GB, GPU desconhecida; desconhecido cai em **Mid**.
3. Governador em execução: janela de 5 s com p95 do frame time; acima de 1,3× do orçamento por 2 janelas, desce um tier; alerta térmico do sistema também desce; nunca sobe sozinho na mesma sessão.
4. Sobrescrita: `GAME_TIER=high|mid|low` e opção "Qualidade dos personagens: Automática / Alta / Média / Baixa" nas configurações.

**Política por avatar** (métrica = altura do avatar na tela em pixels lógicos, porque a câmera é isométrica e tem zoom):

| Tier do aparelho | Jogador local | Próximos (T1 ≈ 220 px) | Médios | Distantes (T2 ≈ 90 px) |
|---|---|---|---|---|
| High | L0 | L0 (até 8 avatares), resto L1 | L1 | L2 |
| Mid | L1 (L0 na criação) | L1 | L1 → L2 por tamanho | L2 |
| Low | L1 | L2 | L2 | L2 (e teto de avatares visíveis, 15.4) |

**Salas com até 50 avatares (decidido, 15.4)**: o servidor e o cliente suportam 50 por sala; quantos são **desenhados** é decidido pelo zoom da câmera (o enquadramento já limita quem está na tela) e por um teto de avatares visíveis por tier que será calibrado na F2 (ainda sem número). Fora do frustum, não se anima nem se faz skinning. Com 50 avatares todos distintos a memória de texturas é o limite: os distantes usam L2 (512 px) e o cache descarrega por LRU. Histerese de 15 % entre níveis e atraso mínimo de 500 ms entre trocas; a troca carrega o novo nível antes de descartar o antigo (o protótipo hoje destrói o corpo e recria; isso precisa mudar). Limiares são hipóteses iniciais para calibrar na F2.

## 7. Animações

- O pacote e o protótipo **não têm animações**. Para o jogo: `idle`, `walk` (com `tiles_per_sec`, sem root motion), `sit`, `lay`, mais os de objetos (seção 8).
- Método do jogo (`build_animations.py`, `rig_math.py`, `sample_mixamo.py`): amostrar com Blender, matemática em Python, `BONE_MAP` novo para os nomes `CC_Base`, correção de direção do osso (A-pose do CC contra T-pose do Mixamo), quadril só com deslocamento vertical escalado pela altura. **Mover essas ferramentas para a crate** (`tools/`), porque retarget é parte do esquema de rigging reutilizável. Fonte dos clips: **Mixamo** (decidido, 15.9): `idle`, `walk`, `sit`, `lay` e beber, retargetados para `CC_Base`.
- **Rigging**: esqueleto mestre único por avatar (71 juntas `CC_Base`, 57 se descartar torção), um `AnimationPlayer` no nó raiz, peças com joints remapeados por nome (o protótipo já faz), cabelo e chapéu com esqueleto próprio pendurados em `CC_Base_Head`; nó raiz padronizado (`Skeleton`) no export; escala 0,01 do FBX aplicada; 4 influências por vértice no export (cabelo longo chega a 16, revisar peça a peça); mesma pose de repouso e inverse bind matrices em todas as peças (`check_skeletons.py` vira teste). Ossos de torção ficam parados.
- **Camadas**: `AnimationGraph` com máscara de corpo superior para o gesto de beber sobre idle/walk/sit.
- **Taxa reduzida**: o `AnimationPlayer` do Bevy avalia todo frame; proposta: sistema próprio que pausa o player dos distantes e o avança por `seek_to` a cada 100 ms (10 Hz) ou 200 ms (5 Hz), escalonado entre avatares, e que não avalia o que está fora do frustum. **Viabilidade de pause + seek_to e ausência de pulo visível são itens de medição da F0/F2**; plano B: interpolar a pose entre amostras ou clip de baixa frequência pré-amostrado.

## 8. Características do personagem: olhos, expressões, piscada, objetos na mão

### 8.1 Rosto

- **Olhos**: o rosto tem peça de olho com máscara própria (`mat_eye`), brilho (`mat_eye_highlight`), cílios, sobrancelhas, boca; no atlas são partes de **1 draw call**. A cor da íris é uma cor de slot do rosto.
- **Piscada**: `FaceState { blink_timer, blink_phase }` sorteia o intervalo (2–6 s), fecha em ~80 ms e reabre em ~120 ms usando o target de olhos fechados (candidato `eyes_closed.01..03`; a conferir na F4) via `MeshMorphWeights` (o protótipo já repassa os pesos do mesh original ao mesh mesclado do atlas). Cada avatar com fase própria; **só em L0/L1**.
- **Expressões**: conjunto inicial `neutro`, `feliz`, `triste`, `bravo`, `surpreso` mapeados nos targets `happy_*`, `sad_*`, `angry_*`, `surprised_01`, mais `eyebrows_up/down` e `jaw_open`; transição suave por tempo; `SetExpression` como mensagem. Cortar do GLB os 10 fonemas e os `eye_look_*` na primeira entrega para reduzir dados (os 46 targets de 2.676 vértices custam ~4 MB por rosto).
- **Direção do olhar**: opcional (`eye_look_*`), só em L0.
- **Ajustes de fisionomia** (`eye_size`, `nose_length`, `tuck_ears`, `shorten`): sliders no criador, valores fixos na aparência.
- Teste: captura por expressão e verificação de que o morph chega ao mesh mesclado com o atlas e em L1.

### 8.2 Pegar objetos (beber num copo ou lata)

- **Sockets**: entidades filhas dos ossos `CC_BaseHand.R/L` (um `GripSocket` por mão) onde o objeto é preso, com **transform de pegada por objeto** guardado nos dados do item (`grip: { socket, translation, rotation, scale, pose }`), ajustado numa ferramenta de pré-visualização do demonstrador.
- **Objeto**: um glTF qualquer (do catálogo de móveis/props); o carregamento é lazy pelo `CdnReader`; em L2 o objeto segurado continua (custa uma draw call) ou some, conforme medição.
- **Animação**: clips `grip_cup`/`grip_can` (pose dos dedos, camada de mão) e `drink` (levar à boca, inclinar, voltar), em camada de corpo superior; fonte Mixamo ("Drinking") retargetada, ou autoria no Blender para o toon. Sequência `Hold → Drink → Release` como máquina de estados da crate; eventos `HoldStarted`, `DrinkFinished`.
- **IK**: o Bevy 0.20 não traz IK; para pegar objetos em alturas variáveis (mesa, balcão) seria preciso IK de dois ossos analítico. **Primeira entrega usa clip fixo** (objeto na altura da mão, como beber); IK entra só se o jogo precisar de "pegar da mesa" (decisão 15.12).
- **Servidor e protocolo (expressões: decidido que vão pelo servidor, 15.5)**: a expressão atual do avatar viaja como campo `expression` (enum pequeno) no `AvatarSnapshot` e na mensagem `avatar_action`, com validação e rate limit no servidor; a piscada é só local. O estado "segurando `item_id`" e o evento de beber precisam chegar aos outros jogadores. Duas opções: (a) só local na primeira entrega (demonstrador e teste), (b) campo opcional `holding` em `AvatarSnapshot` e uma mensagem `avatar_action` (broadcast, rate limit, validado contra o catálogo de props). Recomendo (b) na F6, depois que a F5 estiver validada local. Mesma decisão para expressões (15.5).

## 9. Visual cartoon opcional para salas e objetos

**Requisito**: implementar e **testar** a opção de ligar/desligar o visual cartoon nas salas e nos objetos (móveis, pisos, paredes); os personagens são sempre toon.

- **Estado**: `VisualStyle { Realistic, Cartoon }` como recurso do jogo, ligado a uma opção nas configurações, salva localmente (somente cliente, sem servidor). **Padrão: desligado** (decidido, 15.6); depois dos testes da F7 você decide se passa a ligado.
- **Mecanismo** (duas alternativas, avaliar as duas na F7 e escolher com captura e medição):
  - **A. Troca de material**: um `ToonObjectMaterial` (cor da textura base × rampa de luz, `alpha`, cor de tint, outline opcional) que substitui o `StandardMaterial` de cada malha de cenário, guardando o original (`OriginalMaterial`) para voltar; troca em tempo de execução ao mudar a opção. Mais simples de entender; perde recursos do PBR (sombra recebida, alpha blend dos ghosts) que precisam ser reimplementados.
  - **B. `ExtendedMaterial<StandardMaterial, ToonExt>`**: mantém todo o pipeline PBR (sombras, transparência dos ghosts e das paredes) e aplica o sombreamento toon por uma flag no fragment. Sem troca de handle; a opção só muda um uniform. **Recomendo B como primeira tentativa**, por menor risco com ghosts, paredes translúcidas e sombras.
- **Coerência com os personagens**: a mesma rampa (`ColorRamp`) e a mesma direção de luz, para avatar e cenário combinarem. Outline falso (rim) não funciona bem em caixas e pisos; teste de outline real (casco invertido) só em móveis, atrás da opção (15.7).
- **Onde se aplica no jogo**: `room.rs` (pisos A/B, paredes sólidas e ghost) e `furniture.rs` (modelos glTF, móveis fantasma da pré-visualização do modo construção, cor do móvel); os pontos de criação de material já estão concentrados nesses dois arquivos.
- **Testes** (parte do critério de pronto da F7):
  1. unidade: troca e restauração de material sem vazamento (contagem de handles e `Assets<…>`), ligar/desligar repetidas vezes;
  2. captura de referência de 3 salas (vazia, mobiliada, modo construção com ghost) nos dois estilos, comparadas por diferença de imagem com limiar;
  3. desempenho A/B no iPhone 16 Pro e no S22 (frame time com 20 avatares e 40 móveis);
  4. regressão: com a opção **desligada** não pode haver custo extra nem diferença visual em relação ao estado atual do jogo.

## 10. Personalização, dados e protocolo

- **Slots**: corpo (F/M), rosto (4 por corpo), cabelo (22, com variante de chapéu), chapéu (3), óculos (2), cachecol (3), top (12), bottom (6), conjunto (5, substitui top e bottom), calçado (7), mais roupas customizadas do `fit_garment.py`. Regras do sample de Unity mantidas.
- **Cores**: cada material tem 6 slots (A1, A2, B1, B2, C1, C2 = três regiões × dois tons) mais rim/outline; níveis de personalização: **simples** (1 cor por peça), **intermediário** (3 cores) ou **completo** (6 cores + ajustes do rosto). **Decidido: personalização máxima possível (nível completo)**: 6 cores por peça nos slots editáveis, cor de pele e de íris, ajustes de fisionomia (morphs) e todas as peças e combinações do pacote; a UI do criador agrupa em modo simples e avançado só para não sobrecarregar, sem esconder nada. **Pele = região A do material do corpo** (medido em 30/10, ver 15.13): o tom escolhido muda A2 e A1 é derivado; a região B do corpo é a roupa de baixo embutida e tem cor própria.
- **`AvatarLook`** (crate) é o mesmo dado que o jogo serializa. `AvatarAppearance` v2 no protocolo: `body_type`, `face_id`, `face_morphs` (inteiros 0–255), `slots` com `id` e `colors`, `accessories`; objeto < 600 bytes. Servidor: catálogo v3 (itens com `body_types`, `hides`, `replaces`, materiais e slots editáveis), `Sanitize` validando ids, cores só nos slots editáveis e regras de conjunto/chapéu; fixtures de `contract/`, `CONTRACT_VERSION`, `sync-contract.sh`; testes `go test -race`. **Sem campo de LOD** (é decisão local). Quebra de compatibilidade aceita; aparências antigas viram o visual padrão.
- **Catálogo no formato da crate**: `catalog.json` (assets JSON), com por item: GLB por corpo e por nível, `hides`, `replaces`, `materials[]` (modo, máscara, normal, cores padrão, slots editáveis) e, para props, `grip`. A crate define o **esquema** e o validador; o jogo e o servidor só o consomem.

## 11. Assets e CDN

- **Layout** (mesmo mecanismo da ADR 0011: nome com hash, `manifest.json` por último, `core`/`lazy`, `ttl_days`): `characters/rukha/{base,heads,parts/<categoria>,atlas,textures/<tamanho>}/…` com variantes `l0`, `l1`, `l2` e o campo `lod`/`min_tier` no manifest (apps antigos ignoram). Core: corpo e rosto padrão em L1 e L2 e o atlas do tamanho do tier; tudo mais, lazy.
- **Só baixa o que o tier usa**: aparelho Low nunca baixa L0 nem texturas 2048; é aqui que o ganho em CDN, disco e RAM é maior. Tamanho do protótipo: texturas 2048 = 222 MB para o pacote inteiro; orçamento proposto **≤ 350 KB por peça entregue** e corpo + rosto ≤ 2,5 MB no core, a validar na F0/F8 (ETC1S nas máscaras, UASTC nas normais, normal map descartável por peça no L1/L2).
- **Geometria**: sem Draco; avaliar `KHR_mesh_quantization`/`EXT_meshopt_compression` no loader do Bevy 0.20; remover atributos e morph targets sem uso (o log do protótipo avisa `Unknown vertex attribute COLOR_1..3`).
- **Pipeline**: `tools/` da crate faz FBX→GLB→LODs→texturas→atlas→catálogo; o repositório `resort-game-assets` guarda o **resultado** (assets e manifesto) e continua publicando manualmente (`publish-assets.sh`, `verify-cdn.sh`); os FBX/pacote Unity nunca entram em repositório.
- **`CdnReader`**: a crate carrega por `AssetServer`; o jogo mantém o leitor com espera por arquivo lazy; um item novo pede GLB + texturas do nível.
- **Console de assets** (`resort-game-assets/console`): hoje assume 44 juntas e um corpo. Nada muda na primeira entrega; na F8 ganha aba do pacote, editor de materiais (prévia pelo próprio shader da crate, via exemplo, para não divergir do jogo) e o encaixe de roupas passa a usar o `fit_garment.py` e o esqueleto `CC_Base`.

## 12. Desempenho (iPhone 16 Pro e Galaxy S22)

- **Por avatar**, estimativa após a crate: ≤ 6 draw calls (atlas pele+rosto 1, cabelo, top, bottom, shoes, acessório) contra ~24 sem otimizar e ~7 hoje; triângulos 12–16 mil (L0), 9–11 mil (L1), 4–5 mil (L2) segundo os logs do protótipo; 71 juntas (57 sem torção, ~24 no L2 se a otimização valer).
- **S22 (Adreno 730) com `gpu_compat`** é o risco principal: skinning por uniform buffer e preprocessing em CPU; morph targets, `.basis`, atlas com array de parâmetros no uniform e o `MeshTag` precisam ser testados **nesse modo** (o protótipo nunca foi validado nele, que eu saiba).
- **Memória**: texturas por tamanho (2048² ≈ 2,7 MB comprimida com mipmaps); usar o tamanho do nível do avatar e descarregar o que saiu da sala.
- **Orçamento proposto** (a validar): iPhone 16 Pro ≥ 60 fps e S22 ≥ 45 fps com 20 avatares distintos visíveis, ≤ 300 MB de VRAM do jogo; abaixo disso a ordem é: LOD, resolução, mescla, e só então cortar conteúdo.
- **Medição na F0**: frame time por nível (L0/L1/L2) com 1, 10 e 20 avatares, por aparelho, e custo da animação a 10 Hz.

## 13. Fases, critérios de pronto e estimativa relativa

Tamanhos relativos: P ≈ 1–2 dias, M ≈ 3–5, G ≈ 1–2 semanas, GG ≈ 2+ semanas.

| Fase | Entrega | Pronto quando | Tam. |
|---|---|---|---|
| **F0 Consolidar e medir o protótipo** | `git init` e primeiro commit do protótipo (sem assets, com `.gitignore`), build no iPhone 16 Pro e no S22 com o `gpu_compat` do jogo, relatório de medições | frame time por nível e por quantidade de avatares nos dois aparelhos; morph, `.basis` e atlas funcionando no Adreno 730; viabilidade de `pause + seek_to`; função de cada blend shape documentada; captura lado a lado com o Unity; decisão seguir/ajustar escrita aqui | M |
| **F1 Extrair a crate** | workspace `crates/toon_avatar` + demonstrador; plugins e API da seção 5; catálogos como assets; material **por avatar**; sem globais por nome | demonstrador reproduz o protótipo atual com a crate; teste de dois avatares com cores diferentes sem vazamento; `cargo test` e `clippy` verdes; sem `include_str!` de catálogo | G |
| **F2 LOD por avatar e tier de aparelho** | `DeviceTier`, `QualityProfile`, `AvatarLod`, política da seção 6, troca sem recriar o corpo, animação por taxa (se F3 pronta, senão ganchos) | tabela de tiers e limiares calibrados com a F0; troca de nível sem corpo pelado nem pop; governador desce de tier e nunca sobe (teste); `GAME_TIER` e opção manual | G |
| **F3 Animações** | `idle`, `walk`, `sit`, `lay` retargetados para `CC_Base`, `AvatarAnimationPlugin`, clips + `clips.json` | quatro clips sem interpenetração relevante (conferência visual + relatório), `tiles_per_sec` calibrado, validação de rig em teste; taxa reduzida nos distantes comprovada | G |
| **F4 Rosto** | `AvatarFacePlugin`: piscada, 5 expressões, olhar opcional, ajustes de fisionomia | piscada visível em L0/L1, expressões por captura, morph chega ao mesh do atlas, L2 sem morph e sem custo | M |
| **F5 Objetos na mão** | `AvatarPropsPlugin`: sockets, `grip`, clips de pegada e de beber, máquina de estados | avatar pega copo e lata do catálogo e bebe em idle e sentado, objeto solta ao fim, sem interpenetração grosseira; demonstrador com ferramenta de ajuste do `grip` | G |
| **F6 Integrar ao jogo** | substituir `avatar.rs`/`avatar_anim.rs`/`character_creator.rs` pela crate; protocolo v2, servidor e catálogo v3; `CdnReader`; criador com `bevy_hui`; (se 15.5(b)) mensagem de ação | login → criador → sala com 2 jogadores no iOS e no Android, vendo o mesmo visual, expressões e objetos; aparência antiga abre sem erro; `go test -race`, `check-contract.sh`, `cargo test` verdes | GG |
| **F7 Visual cartoon de salas e objetos** | `ToonStylePlugin`, opção nas configurações, testes da seção 9 | opção liga/desliga sem reiniciar; capturas das 3 salas nos dois estilos; A/B de desempenho nos dois aparelhos; com a opção desligada, nada muda | G |
| **F8 Pipeline, CDN e console** | `tools/` migrados, texturas e LODs por tier, manifest com `lod`/`min_tier`, `verify-cdn`, console adaptado | um comando gera os assets do pacote; peça dentro do orçamento; publicável no DEV em dry-run; console importa e edita materiais | G |
| **F9 (futura) Separar a crate** | `git subtree split` para um repositório privado no GitLab, CI, tags (`v0.x`), docs | jogo compila contra a tag da crate (CI com credencial), histórico sem assets, protótipo antigo arquivado; **fora do escopo imediato** | M |

Dependências: F0 antes de tudo; F1 depois de F0; F2, F3 e F4 sobre a F1 (F3 antes de F5; F4 pode correr junto de F2/F3); F6 depois de F1–F5 (pode começar com F1–F3 e acrescentar o resto); F7 só precisa da F1 (rampa e material) e pode correr em paralelo; F8 acompanha F2/F6; **F9 por último e só quando você decidir separar**. Caminho crítico: F0 → F1 → F3 → F5 → F6. Primeiro marco visível: avatar do pacote andando numa sala do jogo (fim da F3 + parte da F6).

## 14. Riscos

| Risco | Prob. | Impacto | Mitigação |
|---|---|---|---|
| Desempenho no S22 (CPU, 71 juntas, morph, atlas) | alta | alto | medir na F0; mescla/atlas já existentes; L2 com menos juntas; cortar torção |
| Na separação futura, jogo público dependerá de crate privada e terceiros não compilam | certa se mantida | médio | decidir na F9: crate pública só com código, ou CI com credencial |
| A crate acopla ao jogo e a separação futura fica cara | média | médio | regra de fronteira e teste de CI da seção 5.1 |
| `pause + seek_to` não funciona bem no Bevy 0.20 | média | médio | plano B de interpolar pose ou clip de baixa frequência |
| Cores vazando entre avatares (como no protótipo) | certa se não tratada | alto | material por avatar desde a F1, teste automatizado |
| 4 influências estragam cabelo longo e roupas largas | média | médio | revisão peça a peça, reponderar, excluir as ruins |
| Troca de LOD causa pop ou download tardio | média | médio | histerese, carregar antes de descartar, L1/L2 no core |
| Retarget das animações em proporção anime (sentar, deitar, beber) | média | médio | `contact_height`, offsets por osso, relatório de penetração |
| Objeto na mão atravessa o rosto ou a mão | média | médio | `grip` ajustável por item e prévia no demonstrador |
| Cenário toon (opção) destoa ou quebra ghosts/paredes translúcidas | média | médio | alternativa B (`ExtendedMaterial`), capturas A/B, opção desligada por padrão se necessário |
| Texturas pesadas na CDN e na RAM | alta | médio | só baixar o tier do aparelho, ETC1S nas máscaras, TTL |
| Detecção errada do tier | média | baixo | padrão Mid, governador e opção manual |
| Protótipo sem commit e dependente de caminhos locais | certa | médio | F0 |

## 15. Decisões (30/10/2026)

| # | Tema | Decisão | Efeito no plano |
|---|---|---|---|
| 1 | Onde a crate mora | dentro do repositório do jogo; separar no futuro | seção 5.1 e F9 (futura) |
| 2 | Nome | manter os nomes provisórios (`toon_avatar`, `toon_avatar_demo`) e o repositório do jogo, sem repositório novo agora (**interpretei assim a sua resposta**; me corrija se queria outro nome) | seção 5.1 |
| 3 | Corpos | **voltam masculino e feminino** (`body_type`) | seções 3, 10, protocolo v2 |
| 4 | Avatares por sala | **até 50** por sala (salas maiores); visíveis: menos, número ainda não decidido; o zoom controla | seção 6; teto de visíveis calibrado na F2 |
| 5 | Expressões | **pelo servidor** (campo `expression` e mensagem `avatar_action`); piscada local. "Segurando objeto": **segui a recomendação** (também pelo servidor, mesma mensagem, F6) | seções 8.1, 8.2 |
| 6 | Cartoon no cenário | **desligado por padrão**; depois dos testes decide-se se liga | seção 9, F7 |
| 7 | Outline real | seguir a recomendação: testar na F7; só o rim do pacote se o custo não compensar | seção 9 |
| 8 | L3 e juntas reduzidas no L2 | seguir a recomendação: decidir com a medição da F2 | seção 6 |
| 9 | Animações | **Mixamo** | seção 7, F3 |
| 10 | Personalização | **máxima possível** (6 cores por peça, pele, íris, morphs, todas as peças) | seção 10 |
| 11 | Texturas | seguir a recomendação: 1024² máximo no L1; 2048² só no L0 do corpo; normal map só em L0 e L1 | seção 11 |
| 12 | IK | **clips fixos** na primeira entrega | seção 8.2 |
| 13 | Cor da pele | **medido em 30/10/2026**: a pele é a **região A** da máscara (canal R, valor médio ≈ 0,93 em braços, pernas, mãos, pés, ombros, tronco e no rosto, com G e B em zero); a **região B** (canal G) aparece só no quadril (média 0,49) e no tronco (0,56), ou seja, é a **roupa de baixo embutida no corpo**; a C não é usada no corpo. No material F, A1 = (0,54; 0,25; 0,25) é um tom mais escuro e avermelhado (sombra/rubor) e A2 = (0,62; 0,46; 0,41) é o tom de pele; B1 cinza e B2 branco. **Decisão proposta**: o tom de pele escolhido pelo jogador muda **A2** e o **A1 é derivado** automaticamente (mais escuro e saturado); a roupa de baixo (B) fica com cor própria editável. Falta só a conferência visual na F0 | seção 10; F0/F4 |
| 14 | Calçados M | o jogo deve ter calçado masculino: **o pacote não traz `M_shoes`**; os 7 calçados são compartilhados (malha `F_shoes` serve aos dois corpos no sample do Unity) | seção 3; F0 confere o encaixe no corpo M; se não servir, é preciso ajustar as malhas (`fit_garment.py`) |
| 15 | Pacote antigo | **trocar de vez** o Creative Characters (sem flag), no fim da F6 | F6 |
| 16 | Cabelo | **física estática** (cadeias de ossos paradas) | seção 2/6 |
| 17 | Tiers e limiares | seguir a recomendação: valores iniciais calibrados na F0/F2; opção de qualidade visível ao jogador; sem benchmark no primeiro uso | seção 6 |

**Aviso sobre o item 14**: não existe calçado masculino no pacote Rukha93. O jogo masculino usa as mesmas 7 malhas femininas, como o próprio sample do Unity faz. Se na F0 elas ficarem estranhas no corpo M, as opções são ajustar as malhas ao pé masculino ou limitar os calçados no corpo M.

## 16. Fora do escopo

Implementação de qualquer fase, mudanças no jogo, no servidor, nos assets publicados, no protótipo ou no pacote Rukha93, e licenciamento dos pacotes de terceiros (ignorado por ora).

## 17. Limpeza de compatibilidade (pendente)

Decisão de 31/10/2026: **não existe versão antiga publicada do app**, então a compatibilidade com o pacote antigo não tem uso e deve ser removida. Remover, nesta ordem, antes da primeira publicação:

- **Assets (`resort-game-assets`)**: `characters/base/`, `characters/parts/`, `characters/palette.png`, `characters/catalog.json` e `characters/animations/{clips.json,character_anims.glb}` da raiz (os de `F/` e `M/` ficam); entradas correspondentes em `CORE_PREFIXES` e `TTL_DAYS_BY_PREFIX` de `build_manifest.py`; rodar `publish-assets.sh dev --apply --prune` para apagar da CDN o que o manifest não cita mais.
- **Cliente**: o fallback `characters/cosmetics_v3.json` pode virar o único catálogo de primeiro uso; remover o código que lê o formato antigo de aparência (`skin_color`, `hair_color`, `accessories`) e os testes dele.
- **Servidor**: a sanitização de aparência salva antes da v2 (`Catalog.Sanitize`, ramo "aparência anterior à versão 2") e os campos legados do contrato e de `protocol.md`; depois, subir o contrato para a próxima versão e sincronizar o cliente.
- **Manifest**: regra do `AGENTS.md` do repositório de assets ("expanda antes de contrair") deixa de valer até a primeira publicação; reescrever a regra quando houver app publicado.
