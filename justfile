p_args := "-p sec_store -p sec_store_server -p stated_dialogues -p bot -p tui"

run:
    cd bot && cargo run

run-tui:
    cargo run -p tui

format:
    cargo fmt {{p_args}}
    cargo clippy {{p_args}} --fix --allow-dirty

lint:
    cargo fmt {{p_args}} --check
    cargo clippy {{p_args}}

test: lint
    cargo test {{p_args}}

build-release:
    cargo build -p sec_store_server -p bot -p tui --release

run-release: build-release
    ./bot/target/release/bot

run-tui-release: build-release
    ./target/release/tui

docker-build-arm:
    docker build --platform linux/arm64/v8 . --tag passwords_keeper
