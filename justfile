run:
    cd bot && cargo run

format:
    cd sec_store && cargo fmt && cargo clippy --fix --allow-dirty
    cd bot && cargo fmt && cargo clippy --fix --allow-dirty

lint:
    cd sec_store && cargo fmt --check && cargo clippy
    cd bot && cargo fmt --check && cargo clippy

test: lint
    cd sec_store && cargo test
    cd bot && cargo test

build-release:
    cd bot && cargo build --release

run-release: build-release
    ./bot/target/release/bot

docker-build-arm:
    docker build --platform linux/arm64/v8 . --tag passwords_keeper
