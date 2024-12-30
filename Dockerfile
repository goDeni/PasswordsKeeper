FROM --platform=$BUILDPLATFORM rust:1-alpine3.20 AS build_stage

WORKDIR /build

RUN apk add pkgconfig openssl musl-dev libressl-dev

COPY ./Cargo.toml ./Cargo.toml
COPY ./sec_store ./sec_store
COPY ./bot ./bot
COPY ./stated_dialogues ./stated_dialogues

RUN cargo build --bin bot --verbose --release

FROM --platform=$BUILDPLATFORM alpine:3.21.0 AS final_image

COPY \
    --from=build_stage \
    /build/target/release/bot /usr/bin/

WORKDIR /app
VOLUME [ "/app" ]

STOPSIGNAL SIGINT
CMD ["bot"]
