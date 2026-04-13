set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

bootstrap:
    ./docs/scripts/bootstrap.sh

verify:
    ./docs/scripts/verify.sh

hooks:
    ./docs/scripts/install-hooks.sh

db-up:
    ./docs/scripts/dev-db-up.sh

db-down:
    ./docs/scripts/dev-db-down.sh

dev-up:
    ./docs/scripts/dev-up.sh

dev-down:
    ./docs/scripts/dev-down.sh

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

check:
    cargo check --workspace

test:
    ./docs/scripts/test-required.sh

security-test:
    ./docs/scripts/security-report.sh

acceptance:
    ./docs/scripts/v1-acceptance.sh

app:
    cargo run -p memory-app

worker:
    cargo run -p memory-worker
