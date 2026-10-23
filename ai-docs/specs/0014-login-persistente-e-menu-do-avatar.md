# Spec 0014: Login persistente e menu do avatar

## Status
Implemented (não testado em aparelho)

## Objetivo
Quem entrou com Google não precisa entrar de novo a cada abertura do app. O ícone de avatar do HUD abre um menu com **Visuais** (editor do avatar) e **Sair**. O personalizador se comporta diferente na primeira criação e no editor.

## Login persistente
- O `id_token` do Google vale cerca de 1 hora e não é guardado. O que persiste é o **refresh token do Nakama**, salvo em `UserProfile.refresh_token` (`profile.json`, no armazenamento privado do app).
- No `InitAuth`, se o perfil é de Google e tem refresh token, `nakama_client::restore_session` renova a sessão (`/v2/account/session/refresh`), lê `get_profile_status` e segue direto para o criador (sem aparência salva) ou para `Connecting`. Se falhar (token expirado, revogado, sem rede), cai na tela de login.
- A conexão (`ConnectAndJoinPublicRoom`) usa o `id_token` quando existe (login recém-feito) e, senão, o refresh token.
- **Onde fica o token**: no Keychain (iOS, crate `security-framework`) e no Android Keystore (`SecureStore.java`: chave AES-GCM que não sai do Keystore, só o texto cifrado vai para SharedPreferences), via `secure_store.rs`. O `profile.json` só guarda o token em plataformas sem armazenamento seguro (desktop, desenvolvimento). Se o texto cifrado não puder ser lido (backup restaurado em outro aparelho), o app pede login de novo.
- O refresh token é gravado após o login (`check_google_account` devolve `AccountCheck`) e a cada restauração.
- Servidor: `session.refresh_token_expiry_sec` em `local.yml` do resort-game-server sobe para 30 dias (o padrão do Nakama é 1 hora). Trocar `NAKAMA_REFRESH_KEY` desloga todos.

## Menu do avatar (HUD)
- `avatar_menu.rs` (`AvatarMenuPlugin`): o botão de avatar alterna o popup logo abaixo dele, à direita, respeitando a área segura.
- **Visuais**: abre o personalizador em modo editor (`CharacterCreatorState.return_to_room = true`).
- **Sair**: revoga a sessão no servidor (`nakama_client::revoke_session`, `/v2/session/logout`, sem bloquear e sem falhar se estiver offline), `identity::sign_out` apaga `google_account` e o refresh token (perfil e Keychain/Keystore; mantém aparência e `device_id`), derruba o `NakamaResource` e volta para `Login`.
- Um toque fora do menu o fecha e não faz o avatar andar (`input.rs` ignora cliques com o menu aberto).

## Reconexão automática
- Ao voltar do segundo plano (`AppLifecycle` suspenso → ativo) o cliente manda `ForceReconnect`: o socket de um celular que dormiu está morto mesmo quando o sistema ainda não percebeu.
- O worker também envia ping a cada 10 s e derruba a conexão após 30 s de silêncio, e trata queda de socket (fechamento ou erro) do mesmo jeito.
- Reconexão (`nakama_client::reconnect`): novo login com o refresh token (ou conta de dispositivo), novo socket e `match_join` na mesma partida, com até 6 tentativas (0, 1, 2, 4, 8 e 12 s). Um 4xx do servidor (login revogado) não é repetido. O jogador não vê nada; só se todas as tentativas falharem aparece a tela de erro de conexão.
- Servidor (`roommatch/sessions.go`): o avatar só sai da sala quando a última sessão do usuário sai; a sessão antiga, que o servidor só percebe morta depois, não derruba o avatar de quem já reconectou. A posição do avatar é mantida e o snapshot da sala é reenviado.
- Depois da reconexão silenciosa o estado do app continua `InRoom` (nada é reconstruído).

## Personalizador: criação x editor
| | Primeira criação | Editor (HUD → Visuais) |
|---|---|---|
| Botão fechar (X) | oculto | visível, volta ao jogo sem salvar |
| Confirmar | "Confirmar e entrar no jogo" → `Connecting` | "Confirmar e voltar ao jogo" → `InRoom` |

`return_to_room` volta a `false` ao confirmar ou fechar.

## Critérios de Aceite
- [x] Perfil Google com refresh token pula o login (lógica coberta por testes de config/perfil).
- [x] `sign_out` limpa a sessão no perfil e preserva a aparência.
- [x] Rótulo do botão de confirmar muda entre editor e criação.
- [x] Teste do rastreio de sessões no servidor (reconexão não derruba o avatar) e do backoff/erros permanentes no cliente.
- [ ] Verificado em aparelho (Android/iOS): reabrir o app sem logar, Sair → login, editor com X, suspender e voltar sem erro de conexão, token no Keychain/Keystore.
