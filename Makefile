.PHONY: help build release install clean test lint fmt check bench

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build debug binary
	cargo build

release: ## Build release binary
	cargo build --release

install: ## Install to ~/.cargo/bin
	cargo install --path .

clean: ## Remove build artifacts
	cargo clean

test: ## Run tests
	cargo test

bench: ## Run benchmarks
	cargo bench

lint: ## Run clippy
	cargo clippy -- -D warnings

fmt: ## Format code
	cargo fmt

check: fmt lint test ## Format, lint, and test
