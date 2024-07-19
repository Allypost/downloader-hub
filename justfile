set dotenv-load := true
set positional-arguments := true

rustflags := "-C target-feature=+crt-static"
rust_target := "x86_64-unknown-linux-musl"

default:
    @just --list

build:
    RUSTFLAGS='{{rustflags}}' \
    cargo build --release --target '{{rust_target}}'

dev *args: (dev-watch-server args)

dev-watch package *args:
    RUSTFLAGS='{{rustflags}}' \
    cargo watch \
        --clear \
        --quiet \
        --watch './crates' \
        --ignore 'crates/app-migration/**/*' \
        --exec 'run --target "{{rust_target}}" --package "{{package}}" -- {{args}}' \

dev-watch-server *args: (dev-watch 'downloader-hub' args)

dev-watch-cli *args: (dev-watch 'downloader-cli' args)

dev-run package *args:
    RUSTFLAGS='{{rustflags}}' \
    cargo run \
        --target "{{rust_target}}" \
        --package '{{package}}' \
        -- {{args}} \

dev-run-server *args: (dev-run 'downloader-hub' args)

dev-run-cli *args: (dev-run 'downloader-cli' args)

migrate +ARGS: && generate-entities
    cd ./crates/app-migration \
    && cargo run -- "$@" \

migrate-up:
    just migrate up

migration-create migration_name:
    just migrate generate '{{ migration_name }}'

generate-entities:
    sea-orm-cli generate entity \
        --with-copy-enums \
        --with-serde 'serialize' \
        --model-extra-attributes 'serde(rename_all = "camelCase")' \
        --serde-skip-hidden-column \
        --output-dir "./crates/app-entities/src/entities" \

fmt-dev: && fmt
    rustup run nightly cargo fmt --all \

lint:
    cargo clippy --workspace --all-features -- \

lint-fix:
    cargo clippy --fix --allow-dirty --allow-staged --workspace --all-features -- \

fmt: lint-fix
    cargo fmt --all \
