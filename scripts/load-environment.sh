# Sourced by the mobile build scripts. Loads environments/$ENVIRONMENT.env as defaults:
# variables already set in the caller's env (NAKAMA_HOST, ...) win.
# shellcheck shell=bash
load_environment() {
  local name="$1" file="$ROOT/environments/$1.env" line key
  [ -f "$file" ] || { echo "Unknown environment '$name' (expected a file at environments/$name.env)"; exit 1; }
  while IFS= read -r line; do
    case "$line" in ''|'#'*) continue ;; esac
    key="${line%%=*}"
    [ -z "${!key:-}" ] && export "$key=${line#*=}"
  done < "$file"
  [ -n "${NAKAMA_HOST:-}" ] || { echo "environments/$name.env has no NAKAMA_HOST yet; set it there or pass NAKAMA_HOST=..."; exit 1; }
}

# Writes the runtime config the game reads from assets/dev.env (see parse_env_file in lib.rs).
# Needs load_environment to have run. GAME_ENV mirrors $ENVIRONMENT (dev|prod); the server key is
# only written when NAKAMA_SERVER_KEY is set and is never printed.
write_dev_env() {
  local out="$1"
  printf 'NAKAMA_HOST=%s\nNAKAMA_PORT=%s\nGAME_DEV_MODE=%s\nGAME_ENV=%s\n' \
    "$NAKAMA_HOST" "$NAKAMA_PORT" "${GAME_DEV_MODE:-0}" "${ENVIRONMENT:-dev}" > "$out"
  [ -z "${NAKAMA_SERVER_KEY:-}" ] || printf 'NAKAMA_SERVER_KEY=%s\n' "$NAKAMA_SERVER_KEY" >> "$out"
  [ -z "${ASSETS_MANIFEST_URL:-}" ] || printf 'ASSETS_MANIFEST_URL=%s\n' "$ASSETS_MANIFEST_URL" >> "$out"
}

# Stops the build when it would ship the default server key to a remote (TLS) server: the server
# answers 401 "Server key invalid" to every login (Google included). Set ALLOW_DEFAULT_SERVER_KEY=1
# to build anyway.
require_server_key() {
  [ -n "${NAKAMA_SERVER_KEY:-}" ] && return 0
  [ "${ALLOW_DEFAULT_SERVER_KEY:-}" = 1 ] && return 0
  [ "${NAKAMA_PORT:-}" = 443 ] || return 0
  echo "NAKAMA_SERVER_KEY is not set: the app would use 'defaultkey' and ${NAKAMA_HOST:-the server} rejects it (401 Server key invalid)."
  echo "Run: NAKAMA_SERVER_KEY=<key from /opt/resort/.env on the VPS> $0   (or ALLOW_DEFAULT_SERVER_KEY=1 to ignore)"
  exit 1
}
