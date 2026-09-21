# Custos Monorepo Tasks

default:
    @just --list

# Build all workspace crates
build:
    cargo build --workspace

# Fast check without code generation
check:
    cargo check --workspace --all-targets

# Run all unit and integration tests
test:
    cargo test --workspace

# Lint entire workspace with Clippy
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Verify formatting
fmt-check:
    cargo fmt --check --all

# Auto-format all code
fmt:
    cargo fmt --all

# Verify architectural dependency rules
check-deps:
    ./scripts/check_deps.sh
