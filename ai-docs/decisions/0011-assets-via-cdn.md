# ADR 0011: Assets pesados e catálogos fora do app

## Status
Aceita. Em 23/10/2026 o dono decidiu que o app é **CDN-only** (veja "Atualização" no fim). Falta validar em aparelho (spec 0010).

## Contexto
Todo `.glb` e áudio vai dentro do app, então conteúdo novo exige novo build e passagem pela loja. Os catálogos JSON também eram embutidos.

## Decisão
- Catálogos vêm do servidor (`get_catalogs`), com cache em disco por servidor e a cópia embutida só como reserva offline (`catalog_sync.rs`).
- Assets pesados (`audio/`, `characters/`, `furniture/`, exceto `catalog.json` e `clips.json`) vão para o Cloudflare R2: dev em `assets-dev.joaovictor.eng.br`, prod em `assets.joaovictor.eng.br`. UI, fonte e ícones continuam embutidos.
- Arquivos imutáveis com hash no nome (`nome.<sha8>.ext`, cache de 1 ano) e `manifest.json` com cache de 60 s, enviado por último.
- **Publicação sempre manual**: `scripts/publish-assets.sh dev|prod [--apply]` (dry-run por padrão; prod pede confirmação). Não há GitHub Actions.
- O app baixa para o disco (`nakama_client::asset_cache`), verifica sha256 e tamanho, grava de forma atômica e remove versões antigas. Só dados são baixados, nunca código.

## Consequências
- Trocar um `.glb` não exige novo build do app depois que o carregamento via CDN estiver ligado.
- Dev e prod têm buckets, manifests e servidores independentes; o servidor devolve o `assets_manifest_url` do próprio ambiente.
- O `AssetSource` default é substituído por um leitor que consulta o cache e cai para os assets embutidos, então nenhum ponto de carregamento muda e a CDN pode falhar sem quebrar o jogo.
- Enquanto os assets pesados continuarem no bundle, eles são a reserva offline; removê-los é um passo posterior.

## Atualização (23/10/2026): CDN-only
- O app não embute mais personagens, móveis, áudio, miniaturas, `catalog.json` nem `clips.json`: tudo vem da CDN. Só UI, ícones e a fonte (Inter, OFL, com `assets/fonts/OFL.txt`) ficam no app.
- Os assets pesados e o pipeline (`tools/asset_pipeline`, `publish-assets.sh`) vão para o repositório privado `resort-game-assets`; este repositório (que será público) fica sem eles.
- `CdnReader` espera pelos arquivos que ainda estão baixando (consulta o cache a cada 100 ms enquanto a sincronização não terminou ou está sendo repetida), então os pontos de `asset_server.load` continuam iguais e não disputam com o download. Só um CDN `Complete` (ou desativado) resulta em arquivo não encontrado.
- A tela inicial espera o tier `core`. Se a sincronização falha (sem rede e sem cache), mostra "Sem conexão com o servidor de conteúdo" e tenta de novo com intervalo de 5, 10, 20 e 30 s; não há mais reserva embutida. A primeira abertura exige rede.
- `clips.json` e `characters/catalog.json` são lidos como `JsonText` pelo `AssetServer`. O catálogo de cosméticos vindo do servidor (ou do cache) tem prioridade; o `characters/catalog.json` da CDN só cobre a primeira abertura, em que o criador de personagem abre antes da conexão.
- `CharacterCatalog::default()` é vazio. Testes usam fixtures sintéticas em `crates/game/tests/fixtures/`. As verificações contra os arquivos reais (raiz do rig, clips vs. biblioteca de animação, arquivos do catálogo) passaram para `resort-game-assets` (`test_assets_consistency.py`).

## Atualização (24/10/2026): tier `lazy` sob demanda e validade
- O app baixa de início só o tier `core` (corpo base, animações, paleta e catálogos). Peças de roupa, móveis, miniaturas e áudio são `lazy`: o `CdnReader` pede o arquivo à thread de sincronização na primeira leitura, espera a chegada e repete o pedido após 30 s se falhar.
- Cada arquivo `lazy` traz `ttl_days` no manifest: 7 para `furniture/`, 30 para `characters/parts/`, `ui/thumbs/` e `audio/`. Arquivos `core` não expiram. A validade conta desde o último uso (a leitura renova o `mtime` se tiver mais de um dia) e `prune_expired` apaga o que passou do prazo.
- A lista de roupas vem do servidor: `get_catalogs` roda junto da verificação de conta (login e restauração de sessão), antes de o criador abrir. O cache por servidor e o `characters/catalog.json` da CDN continuam como reserva.
- No criador, trocar só cores repinta as peças já carregadas em vez de recriar o avatar; as cenas das peças já vistas ficam retidas para não reler o `.glb` ao voltar a uma peça.
