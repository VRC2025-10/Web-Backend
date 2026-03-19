# =============================================================================
#  Makefile — VRC Web-Backend
#  Unified task runner for development, testing, and maintenance.
# =============================================================================

.DEFAULT_GOAL := help
SHELL := /bin/bash

# ── Variables ────────────────────────────────────────────────────────────────
CARGO := cargo
DOCKER_COMPOSE := docker compose
DOCKER_COMPOSE_PROD := docker compose -f docker-compose.prod.yml

# ── Help ─────────────────────────────────────────────────────────────────────

.PHONY: help
help: ## Show this help message
	@echo "VRC Web-Backend — Development Commands"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# ── Development ──────────────────────────────────────────────────────────────

.PHONY: setup
setup: ## Install dependencies and set up the development environment
	@echo "Checking Rust toolchain..."
	rustup show
	@echo ""
	@echo "Installing development tools..."
	rustup component add rustfmt clippy
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo "Created .env from .env.example — please update with your settings"; \
	fi
	@echo ""
	@echo "Starting PostgreSQL..."
	$(DOCKER_COMPOSE) up -d
	@echo ""
	@echo "Building project..."
	$(CARGO) build
	@echo ""
	@echo "Setup complete! Run 'make run' to start the server."

.PHONY: run
run: ## Run the development server
	$(CARGO) run

.PHONY: run-release
run-release: ## Run the server in release mode
	$(CARGO) run --release

.PHONY: watch
watch: ## Run with auto-reload (requires cargo-watch)
	cargo watch -x run

# ── Build ────────────────────────────────────────────────────────────────────

.PHONY: build
build: ## Build in release mode
	$(CARGO) build --release

.PHONY: build-debug
build-debug: ## Build in debug mode
	$(CARGO) build

# ── Testing ──────────────────────────────────────────────────────────────────

.PHONY: test
test: ## Run all tests
	$(CARGO) test

.PHONY: test-verbose
test-verbose: ## Run all tests with verbose output
	$(CARGO) test -- --nocapture

# ── Code Quality ─────────────────────────────────────────────────────────────

.PHONY: lint
lint: ## Run clippy and format check
	$(CARGO) fmt --check
	$(CARGO) clippy -- -D warnings

.PHONY: fmt
fmt: ## Auto-format code
	$(CARGO) fmt

.PHONY: clippy
clippy: ## Run clippy lints
	$(CARGO) clippy -- -D warnings

.PHONY: check
check: lint test build ## Run full pre-commit check (lint + test + build)
	@echo "All checks passed!"

# ── Database ─────────────────────────────────────────────────────────────────

.PHONY: db-up
db-up: ## Start PostgreSQL via Docker Compose
	$(DOCKER_COMPOSE) up -d

.PHONY: db-down
db-down: ## Stop PostgreSQL
	$(DOCKER_COMPOSE) down

.PHONY: db-reset
db-reset: ## Reset database (stop, remove volume, start fresh)
	$(DOCKER_COMPOSE) down -v
	$(DOCKER_COMPOSE) up -d
	@echo "Database reset. Migrations will run on next server start."

.PHONY: db-logs
db-logs: ## Show PostgreSQL logs
	$(DOCKER_COMPOSE) logs -f postgres

# ── Docker ───────────────────────────────────────────────────────────────────

.PHONY: docker-build
docker-build: ## Build the production Docker image
	docker build -t vrc-backend:latest .

.PHONY: docker-up
docker-up: ## Start all services (production compose)
	$(DOCKER_COMPOSE_PROD) up -d

.PHONY: docker-down
docker-down: ## Stop all services (production compose)
	$(DOCKER_COMPOSE_PROD) down

.PHONY: docker-logs
docker-logs: ## Show production container logs
	$(DOCKER_COMPOSE_PROD) logs -f app

# ── SQLx ─────────────────────────────────────────────────────────────────────

.PHONY: sqlx-prepare
sqlx-prepare: ## Regenerate SQLx offline query cache
	cargo sqlx prepare --workspace

# ── Maintenance ──────────────────────────────────────────────────────────────

.PHONY: clean
clean: ## Clean build artifacts
	$(CARGO) clean

.PHONY: update
update: ## Update Cargo dependencies
	$(CARGO) update

.PHONY: audit
audit: ## Run security audit on dependencies
	cargo audit

.PHONY: deny
deny: ## Run cargo-deny checks (licenses, advisories, bans)
	cargo deny check
