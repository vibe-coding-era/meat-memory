set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

bootstrap:
    ./scripts/bootstrap.sh

verify:
    ./scripts/verify.sh

hooks:
    ./scripts/install-hooks.sh

db-up:
    ./scripts/dev-db-up.sh

db-down:
    ./scripts/dev-db-down.sh

dev-up:
    ./scripts/dev-up.sh

dev-down:
    ./scripts/dev-down.sh

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

check:
    cargo check --workspace

test:
    cargo test --workspace

acceptance:
    ./scripts/v1-acceptance.sh

app:
    cargo run -p memory-app

worker:
    cargo run -p memory-worker
