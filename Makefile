.PHONY: help client check test sync-contract stack-up stack-down stack-logs

help:
	@echo "make client         - executa o cliente Bevy no desktop"
	@echo "make check          - fmt, clippy, testes e drift do contrato"
	@echo "make test           - cargo test --workspace"
	@echo "make sync-contract  - copia o contrato de ../resort-game-server/contract"
	@echo "make stack-up       - sobe Nakama + PostgreSQL locais (imagem do GHCR)"
	@echo "make stack-down     - derruba a stack local"
	@echo "make stack-logs     - logs do Nakama"

client:
	cargo run -p game

check:
	./scripts/check.sh

test:
	cargo test --workspace

sync-contract:
	./scripts/sync-contract.sh

stack-up:
	docker compose -f docker-compose.local.yml up -d

stack-down:
	docker compose -f docker-compose.local.yml down

stack-logs:
	docker compose -f docker-compose.local.yml logs -f nakama
