# Spec 0002: Gerenciamento de Nickname e Identidade Local

## Status
Approved

## Contexto e Objetivo
Prover um sistema de identidade de usuário que funcione sem atrito em produção e facilite o desenvolvimento local com múltiplas sessões.
- **Em Produção**: O usuário define seu apelido (nickname) na primeira execução. Os dados de identidade (`device_id` e `nickname`) são persistidos localmente no dispositivo para login silencioso automático. O modelo segue o padrão *Guest-first*, compatível com futura vinculação (*Account Linking*) a Apple ID e Google Sign-In no Nakama.
- **Em Desenvolvimento**: Permite abrir simultaneamente múltiplos clientes na mesma máquina sem colisão de IDs ou de nomes de usuário, utilizando flags de linha de comando (`--nick`, `--profile`, `--dev`) ou variáveis de ambiente.

---

## Escopo

### Dentro do Escopo
1. **Validação de Nickname**:
   - Comprimento entre 3 e 16 caracteres.
   - Caracteres permitidos: alfanuméricos e sublinhado (`^[a-zA-Z0-9_]{3,16}$`).
2. **Resolução Hierárquica de Identidade**:
   - `CLI Args` (`--nick`, `--profile`, `--dev`, `--device-id`) > `ENV Vars` (`GAME_USERNAME`, `GAME_DEVICE_ID`, `GAME_PROFILE`) > `Local Storage` (`profile_<name>.json` ou `profile.json`) > `Modo Efêmero/Prompt`.
3. **Persistência de Perfil Local**:
   - Estrutura `UserProfile` com `device_id`, `nickname` e `created_at`.
   - Serialização e desserialização em JSON.
   - Salvamento em diretório de dados da aplicação no sistema operacional ou relativo ao perfil informado.
4. **Fluxo de Estados no Bevy**:
   - `AppState::InitAuth`: avalia se a identidade já está resolvida ou se necessita intervenção.
   - `AppState::ChooseNickname`: interface inicial de entrada de nickname (quando não houver perfil prévio).
   - `AppState::Connecting`: inicia conexão ao Nakama com a identidade estabelecida.
5. **Compatibilidade Futura com Nakama Account Linking**:
   - Uso de `authenticate_device` com o `device_id` persistido e o `username` registrado, permitindo posterior chamada a `/v2/account/link/apple` ou `/v2/account/link/google`.

### Fora do Escopo
- Integração direta com SDKs de Apple ID ou Google Sign-In (será implementado em especificação posterior).
- Moderação de conteúdo ou dicionário de palavras ofensivas (será validado no servidor em milestone futuro).

---

## Requisitos Técnicos e Componentes Bevy

### 1. Módulo `identity`
- **`struct UserProfile`**:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct UserProfile {
      pub device_id: String,
      pub nickname: String,
      pub created_at: u64,
  }
  ```
- **`fn validate_nickname(nick: &str) -> Result<(), NicknameError>`**:
  - `Empty`, `TooShort`, `TooLong`, `InvalidCharacters`.
- **`struct IdentityConfig` / `GameConfig`**:
  - Analisa `std::env::args()` e `std::env::vars()`.

### 2. Estados da Aplicação (`AppState`)
- `AppState::InitAuth` (default inicial):
  - Sistema `resolve_identity_system` avalia se a identidade já está disponível.
  - Se sim -> transiciona para `AppState::Connecting`.
  - Se não -> transiciona para `AppState::ChooseNickname`.
- `AppState::ChooseNickname`:
  - Renderiza formulário para inserção do nickname com validação em tempo real.
  - Ao submeter nickname válido, salva `profile.json` e transiciona para `AppState::Connecting`.
- `AppState::Connecting`:
  - `setup_network` inicia a conexão com Nakama com base na identidade validada.

---

## Critérios de Aceite (Testáveis)

- [ ] `validate_nickname` aceita nomes válidos (ex: `"Alice"`, `"Player_1"`, `"neo_2026"`).
- [ ] `validate_nickname` rejeita nomes menores que 3 caracteres, maiores que 16 caracteres ou com caracteres inválidos (espaços, pontuação, símbolos especiais).
- [ ] `UserProfile::save` e `UserProfile::load` gravam e recuperam com sucesso os dados em disco.
- [ ] A resolução de configuração prioriza argumentos CLI sobre variáveis de ambiente e arquivo local.
- [ ] A flag `--profile <nome>` carrega/salva em arquivo de perfil isolado (ex: `profile_nome.json`).
- [ ] A flag `--dev` gera credenciais efêmeras sem gravar no armazenamento persistente de produção.
- [ ] Se houver perfil salvo, o cliente pula a tela `ChooseNickname` e conecta diretamente.
- [ ] Se não houver perfil nem flags fornecidas, o cliente entra no estado `ChooseNickname`.
- [ ] Testes unitários com cobertura em `validate_nickname`, resolução de identidade e serialização de perfil.

---

## Plano de Testes e Validação
1. **Testes Unitários**:
   - `test_validate_nickname_valid`: casos válidos.
   - `test_validate_nickname_errors`: tamanho e caracteres inválidos.
   - `test_profile_persistence`: gravação em arquivo temporário e leitura com paridade.
   - `test_identity_resolution_hierarchy`: CLI > ENV > Storage.
2. **Validação de Build**:
   - Execução de `./scripts/check.sh` no client com zero warnings ou erros de clippy.
