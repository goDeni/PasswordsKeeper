run:
    cd bot && cargo run

run-tui:
    cargo run -p tui

format:
    cd sec_store && cargo fmt && cargo clippy --fix --allow-dirty
    cd sec_store_server && cargo fmt && cargo clippy --fix --allow-dirty
    cd bot && cargo fmt && cargo clippy --fix --allow-dirty
    cd stated_dialogues && cargo fmt && cargo clippy --fix --allow-dirty
    cd tui && cargo fmt && cargo clippy --fix --allow-dirty

lint:
    cd sec_store && cargo fmt --check && cargo clippy
    cd sec_store_server && cargo fmt --check && cargo clippy
    cd bot && cargo fmt --check && cargo clippy
    cd stated_dialogues && cargo fmt --check && cargo clippy
    cd tui && cargo fmt --check && cargo clippy

test: lint
    cargo test -p sec_store
    cargo test -p sec_store_server
    cargo test -p bot
    cargo test -p stated_dialogues
    cargo test -p tui

build-release:
    cd bot && cargo build --release
    cd sec_store_server && cargo build --release
    cd tui && cargo build --release

run-release: build-release
    ./bot/target/release/bot

run-tui-release: build-release
    ./target/release/tui

docker-build-arm:
    docker build --platform linux/arm64/v8 . --tag passwords_keeper
