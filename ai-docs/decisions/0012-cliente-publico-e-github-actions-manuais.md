# ADR 0012: Cliente público, assets só pela CDN e GitHub Actions manuais

## Status
Aceita (23/10/2026). Reverte a decisão "sem GitHub Actions" (commit `dc6df6d`) **apenas no cliente**; o servidor continua sem Actions.

## Contexto
O cliente vai ficar público. Pacotes de terceiros (personagens, móveis) não podem ficar no repositório nem no histórico, e os builds de IPA e APK passaram a precisar de um ambiente reproduzível. Plano completo: `plans/cliente-publico-e-assets-privados.md` (pasta de arquivos do projeto).

## Decisão
1. Assets pesados, `catalog.json`, `clips.json` e o pipeline moram em `resort-game-assets` (privado), publicados na R2 por script manual. O app os consome **só pela CDN** (ADR 0011).
2. Actions no cliente, em três workflows:
   - `check.yml`: `scripts/check.sh` em PR e em push na `develop`/`main`. Sem secrets, então roda também em PRs de forks.
   - `build-android.yml` e `build-ios.yml`: **somente `workflow_dispatch`**, com a escolha de `dev` ou `prod`. Cada job declara `environment:`; os secrets (`NAKAMA_SERVER_KEY`, keystore Android opcional, bucket privado de builds) existem só no Environment.
3. **O IPA sai sem assinatura** por enquanto (`SIGNING=none` em `scripts/export-ipa.sh`): compila e empacota, mas só instala depois de assinado. Assinar no runner (certificado `.p12`, perfil e chave da App Store Connect como secrets do Environment) é uma etapa futura que não exige reescrever o workflow.
4. Repositório público: binários de prod nunca viram artifact do GitHub (artifacts e logs são públicos); vão para um bucket privado (`BUILDS_R2_*`). Dev pode usar um artifact de 1 dia quando não há bucket.
5. Regras do repositório (configuradas no GitHub): aprovação de workflows de colaboradores externos, `GITHUB_TOKEN` somente leitura, sem `pull_request_target`, sem runners próprios, Environment `prod` com revisor e restrição de branch.
6. Commit único como início do histórico público; sem licença de uso aberta; Git LFS não é usado.

## Consequências
- A regra "sem GitHub Actions" do `AGENTS.md` do cliente muda para "só os workflows listados acima".
- A primeira abertura do app exige rede (sem reserva embutida).
- Versões de actions em tag (`@v4`): fixar por SHA quando o Dependabot abrir os PRs de atualização.
