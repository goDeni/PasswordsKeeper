run:
    cd bot && cargo run

format:
    cd bot && cargo fmt && cargo clippy --fix --allow-dirty
    cd sec_store && cargo fmt && cargo clippy --fix --allow-dirty

lint:
    cd bot && cargo fmt --check && cargo clippy
    cd sec_store && cargo fmt --check && cargo clippy

test: lint
    cd bot && cargo test
    cd sec_store && cargo test

build-release:
    cd bot && cargo build --release

run-release: build-release
    ./bot/target/release/bot

docker-build-arm:
    docker build --platform linux/arm64/v8 . --tag passwords_keeper