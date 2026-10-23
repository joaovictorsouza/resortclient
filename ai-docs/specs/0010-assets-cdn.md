# Spec 0010: Assets na CDN (R2) e catálogos do servidor

## Já implementado
- [x] `get_catalogs` consumido no login, com cache em disco (`catalogs-<host>.json` ao lado do perfil) e reserva embutida (`catalog_sync.rs`).
- [x] `tools/asset_pipeline/build_manifest.py`: gera os arquivos com hash e o `manifest.json` (tiers `core` e `lazy`); falha sem asset `core`.
- [x] `scripts/publish-assets.sh`: envio incremental ao R2 (só arquivos novos), manifest por último, dry-run por padrão.
- [x] `nakama_client::asset_cache`: parser do manifest (recusa caminhos inseguros), verificação de hash, download atômico, `missing`, `prune`.

## Carregamento pela CDN (protótipo implementado)
- [x] `game/src/cdn_assets.rs`: o `AssetSource` **default** do Bevy 0.20 é registrado antes do `DefaultPlugins` (o `AssetPlugin` só cria o default se ele não existir) e embrulhado por `CdnReader`. Um caminho lógico (`characters/base/body_010.glb`) é servido do arquivo baixado e verificado quando está no índice, e dos assets embutidos no app caso contrário. Os pontos de `asset_server.load` **não mudam**.
- [x] `nakama_client::asset_cache::sync_blocking` (thread própria): busca o manifest (ou usa o último salvo, offline), anuncia os arquivos válidos do cache, baixa o tier `core`, libera o jogo (`CoreReady`) e baixa o tier `lazy` em segundo plano; apaga versões antigas.
- [x] A tela inicial (`InitAuth`) espera o `core` e mostra "Baixando conteúdo… n/m"; se falhar ou não houver URL, segue com os assets embutidos.
- [x] URL do manifest: `assets_manifest_url` do último `get_catalogs` em cache, senão `ASSETS_MANIFEST_URL` (de `environments/*.env`, gravado em `assets/dev.env` pelos scripts iOS e pelo Gradle).
- [x] Cache: `<pasta do perfil>/assets-cache/<host da CDN>/`.
- [x] Móveis novos sem código: `build_manifest.py` grava `model: {size, bottom}` (caixa do `.glb`) para `furniture/*.glb`; o app registra o modelo (`furniture::register_cdn_model`) e o ajusta ao `width` x `depth` do item (`furniture_model`). Móveis e `ui/thumbs` entram no tier `core`. Limitação: a caixa ignora transformações de nós do glTF.
- [x] Testes: leitor (cache, fallback, arquivo sumido), `AssetServer` real lendo pelo source registrado, e ponta a ponta contra um servidor HTTP local (core + lazy, segunda execução sem downloads, arquivo corrompido recusado, 404, offline com manifest salvo).

## Pendente
- [ ] Validar em aparelho (iOS e Android) com buckets reais: pasta de dados e tempo de download não foram exercitados fora dos testes.
- [x] Código CDN-only (23/10/2026): leitura que espera o download, retry com backoff na tela inicial, `clips.json` e catálogo da CDN, fonte Inter embutida, catálogo vazio por padrão.
- [ ] Remover os arquivos pesados do repositório (Fase 2b): só depois da CDN publicada e do app validado em aparelho (cache vazio, modo avião, manifest fora do ar, arquivo corrompido).
- [ ] Baixar sob demanda itens `lazy` novos durante a partida (hoje só no início).
- [ ] Criar os buckets R2 e os domínios (ação sua no Cloudflare).

## Como publicar (manual)
1. Dev: `scripts/publish-assets.sh dev` (confere), depois `scripts/publish-assets.sh dev --apply`. Arquivos antigos ficam no bucket (úteis para apps antigos e rollback); `--prune` apaga os que o novo manifest não usa mais.
2. Prod: o mesmo com `prod`; pede para digitar `prod`.
3. Rollback: reenviar um `manifest.json` anterior (`--version` antigo gera outro manifest; o mais simples é republicar do commit antigo).
