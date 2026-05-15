# ed2kIA — Developer Experience (DX) Justfile
# Usage: just <recipe>
# List recipes: just

# ─── Setup ───

setup: ## Install Rust toolchain & dependencies
	@echo "Setting up ed2kIA development environment..."
	@rustup default stable
	@rustup component add clippy rustfmt
	@cargo install just
	@echo "Setup complete!"

setup-full: setup ## Install all optional tooling
	@rustup target add wasm32-unknown-unknown
	@cargo install cargo-audit cargo-expand cargo-nextest
	@echo "Full setup complete!"

# ─── Build ───

build: ## Build (debug, CPU)
	@cargo build

build-release: ## Build (release, CPU)
	@cargo build --release

build-cuda: ## Build with CUDA GPU support
	@cargo build --features cuda

build-metal: ## Build with Metal (Apple Silicon)
	@cargo build --features metal

build-wasm: ## Build for WASM target
	@cargo build --target wasm32-unknown-unknown --features v1.8-sprint2

# ─── Validate ───

check: ## Syntax check
	@cargo check --features stable

check-all: ## Check all features
	@cargo check --all-features

clippy: ## Lint with Clippy
	@cargo clippy --features stable -- -D warnings

clippy-fix: ## Auto-fix Clippy warnings
	@cargo clippy --features stable --fix --allow-dirty -- -D warnings

fmt: ## Format code
	@cargo fmt --all

fmt-check: ## Check formatting
	@cargo fmt --all -- --check

# ─── Test ───

test: ## Run unit tests
	@cargo test --features stable

test-fast: ## Run only lib tests (faster)
	@cargo test --lib --features stable

test-coverage: ## Run tests with coverage (requires tarpaulin)
	@cargo tarpaulin --features stable --out Html

test-sprint2: ## Run v1.8 Sprint 2 tests
	@cargo test --features v1.8-sprint2

# ─── Dev Server ───

dev: ## Run local dev node
	@cargo run -- --port 9000 --dev

dev-watch: ## Run with file watcher (requires cargo-watch)
	@cargo watch -x "run -- --port 9000 --dev"

# ─── Docker ───

docker-build: ## Build Docker image
	@docker build -t ed2kia:latest -f deploy/Dockerfile .

docker-run: ## Run local Docker container
	@docker run --rm -p 9000:9000 ed2kia:latest

docker-compose: ## Start full local dev environment
	@docker compose -f devtools/docker-compose.yml up -d

docker-compose-down: ## Stop local dev environment
	@docker compose -f devtools/docker-compose.yml down

# ─── Benchmarks ───

bench: ## Run benchmarks
	@cargo bench --package benchmarks

bench-tensor: ## Tensor serialization benchmark
	@cargo bench --package benchmarks tensor_serialization

bench-sae: ## SAE loader benchmark
	@cargo bench --package benchmarks sae_loader

# ─── Release ───

release-check: ## Pre-release validation
	@cargo check --release
	@cargo clippy --release -- -D warnings
	@cargo test --release

release-pack: ## Package release artifacts
	@bash release/packager.sh

# ─── Clean ───

clean: ## Clean build artifacts
	@cargo clean

clean-all: clean ## Clean everything including Docker
	@docker compose -f devtools/docker-compose.yml down -v 2>/dev/null || true
	@rm -rf target/

# ─── Info ───

info: ## Show project info
	@cargo --version
	@rustc --version
	@echo "ed2kIA $(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"

help: ## List available recipes
	@just --list
