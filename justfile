run:
    cd bot && cargo run

test:
    cd bot && cargo test
    cd sec_store && cargo test

build-release:
    cd bot && cargo build --release

run-release: build-release
    ./bot/target/release/bot