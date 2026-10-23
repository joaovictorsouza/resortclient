# Investigação: avatar deformado no Android (skinning)

Status: **resolvido com workaround** (2026-10-19, validado no Galaxy S22+). Causa identificada: bug conhecido do Bevy, issue
[bevyengine/bevy#24926](https://github.com/bevyengine/bevy/issues/24926) (skinned GLB corrompido em
Android/Vulkan/Adreno no caminho de skinning por **storage buffer**; aberto, sem PR, afeta 0.19.0, 0.19.1
e `main`). Reproduz também em Adreno 650. Workaround confirmado na issue: forçar o caminho de skinning por
**uniform buffer**. As seções abaixo guardam o histórico; onde divergirem desta, vale esta.

> Correção: a hipótese do "caminho de CPU do preprocessing" estava **errada** (era só correlação: o mesmo
> filtro de Adreno ≤ 730 desliga o preprocessing em GPU). E a linha "falta de storage buffers" da tabela
> abaixo foi mal interpretada: o problema é justamente o caminho de storage buffer ser usado.

## Sintoma

No Android (Galaxy S22+ `SM-S906E`, Android 16, Adreno 730, Vulkan) o lobby, o HUD e a conexão com o
Nakama funcionam, mas o **avatar renderiza deformado**: vértices esticados em listras longas, como se
as matrizes dos ossos estivessem erradas. No iOS (Metal) e no desktop o mesmo avatar renderiza correto.
O defeito é estático (não é glitch de frame) e afeta o corpo e as partes (cabelo, roupa).

## Como reproduzir

```bash
# servidor Nakama no ar (docker compose up -d no repositório resort-game-server), celular por USB com depuração ativa
scripts/run-android.sh
```

O script monta o APK sem Gradle e usa `adb reverse tcp:7350` para o celular alcançar o Nakama do Mac.
`GAME_DEV_MODE=1` é gravado no `assets/dev.env` (pula a tela de apelido). Ver
[platforms.md](../architecture/platforms.md).

## O que já foi descartado

| Hipótese | Resultado |
|---|---|
| Animação / ossos remapeados (`rebind_part_joints`) | Descartada: com `GAME_NO_ANIM=1` a pose de repouso **já** aparece quebrada (pior: só listras). |
| Falta de storage buffers (skin via uniform buffer) | Descartada: limites da GPU mostram `max_storage_buffers_per_shader_stage: 32`, `min_storage_buffer_offset_alignment: 64`; `skins_use_uniform_buffers` é falso. |
| Erro de render/wgpu | Descartada: nenhum ERROR/WARN relevante no logcat (só `Unknown vertex attribute COLOR_1`, `Gilrs`, avisos do winit). |
| Ordem entre `extract_skins` e o batching de CPU | Descartada pela leitura do código do Bevy 0.19.1: o offset do skin é alocado na extração, antes do batching. |
| Bug conhecido do Bevy | Busca na web não achou nada específico (só relatos de Unity/Babylon em GPUs Android). |

## Fatos relevantes

- O Bevy 0.19.1 **desliga o preprocessing em GPU** para este aparelho
  (`is_non_supported_android_device` em `bevy_render/src/batching/gpu_preprocessing.rs`) e usa o caminho
  de **CPU**: log `GPU preprocessing is not supported on this device. Falling back to CPU preprocessing.`
  No iOS o preprocessing roda na GPU. É a diferença de caminho mais provável para skinned meshes
  (o `current_skin_index` do `MeshUniform` é montado em `mesh.rs` ~linha 2842 no caminho de CPU).
- Cada avatar é montado de várias cenas glTF (corpo + partes) que dividem o esqueleto `rig_kid`
  (ver `avatar_anim.rs`). Hoje as partes são re-apontadas para os ossos do corpo.
- Parâmetros do Android: `android-native-activity`, `MIN_SDK=26` (AAudio), `cargo ndk -P 26`.

## Resultado em um segundo aparelho (2026-10-19)

Galaxy A56 (`SM-A566E`, Exynos 1580, GPU Samsung Xclipse 540, Android 16, Vulkan):

- Log: `GPU preprocessing is fully supported on this device.` (caminho de GPU, como no iOS).
- **O avatar renderiza correto e animado.**
- `DBGSKIN` (CPU): `joints=79 bindposes=79 max_translation=9.828 bad=0 missing_global=0`, sem valores
  inválidos nem joints sem `GlobalTransform`.

Leitura: o dado de skin calculado na CPU é são, e o defeito não aparece num aparelho que usa o caminho de
GPU. Isso reforça a hipótese do caminho de CPU/driver do Adreno 730, mas é correlação (2 aparelhos, 2
GPUs diferentes): falta rodar o mesmo `DBGSKIN` no S22+ e fazer o teste de forçar o caminho de GPU nele.

## Solução aplicada (workaround)

`crates/game/src/gpu_compat.rs`, ligado em `run()` via `RenderPlugin`: antes de o renderer subir, o app lê
o nome dos adaptadores Vulkan (`wgpu`, só Android) e, se algum for uma das Adrenos **listadas na issue
(650 e 730)**, aplica `WgpuSettings.constrained_limits` zerando os storage buffers e os limites de compute.
O Bevy passa a ver um dispositivo sem storage buffers e usa skinning por uniform buffer (e preprocessing em
CPU), o caminho que a issue #24926 confirma como correto.

- Gate por **modelo** (`AFFECTED_ADRENO_MODELS = [650, 730]`), não por fabricante. Outras Adrenos (720, 740,
  660...) não são reportadas nem como afetadas nem como funcionando, e ficam **sem** o workaround. Se uma
  delas mostrar o defeito, é só acrescentar o modelo na lista.
- Validado no **S22+ (Adreno 730)**: avatar íntegro parado e andando; log com
  `max_storage_buffers_per_shader_stage: 0`.
- Não afetados pelo gate: Xclipse (A56), Mali, iOS e desktop. O A56 não foi revalidado após a mudança.
- Custos do workaround nas Adrenos gated: sem compute passes do Bevy, `Sparse buffer updates disabled` e
  `OrderIndependentTransparencyPlugin not loaded` (avisos esperados) e limites do wgpu no máximo os
  `Limits::default()` (ex.: textura 8192). Não usamos OIT nem texturas maiores que isso.
- Alternativa sugerida na issue (não adotada): gate por faixa, `get_adreno_model` ≤ 730 excluindo 720,
  igual ao filtro de preprocessing do Bevy. Cobriria modelos não reportados.
- Remover `gpu_compat.rs` (e a dependência `wgpu` de Android em `Cargo.toml`) quando a issue for corrigida.

O código temporário de diagnóstico (`GAME_NO_ANIM`, `dbg_skin_matrices`) foi removido. O
`scripts/run-android.sh` mantém `EXTRA_DEV_ENV` e aceita `ANDROID_SERIAL` para escolher o aparelho.

## Próximos passos

1. Revalidar o A56 (e, se possível, um Adreno mais novo, 740+) com a build atual.
2. Comentar na issue [#24926](https://github.com/bevyengine/bevy/issues/24926) com os dados: S22+ (Adreno 730,
   Android 16, Bevy 0.19.1) reproduz; A56 (Xclipse 540) não; e o gate por modelo de Adreno.
3. Acompanhar a issue e remover o workaround quando houver correção.

## Estado do repositório

Nada foi commitado. Há também outras mudanças desta sessão ainda não commitadas: suporte iOS/Android
(`ios/`, `android/`, `scripts/run-ios-sim.sh`, `scripts/run-android.sh`), `ui_scale.rs`, janela
fullscreen no mobile e `platforms.md`.

## Atualização (2026-10-21): troca do rig

Os personagens passaram para o pacote *Creative Characters* ([ADR 0010](../decisions/0010-creative-characters-pack-and-single-skeleton.md)): o esqueleto agora tem 44 joints (antes, 80) e o nó raiz se chama `Skeleton` (antes, `rig_kid`). O workaround de `gpu_compat.rs` continua necessário (o bug é do caminho de skinning por storage buffer, não do rig). Ainda **não foi revalidado em aparelho** com o rig novo: reexecutar `scripts/run-android.sh` no S22+ e no A56.

