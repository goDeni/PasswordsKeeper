FROM --platform=$BUILDPLATFORM rust:1-alpine3.18 as build_stage

WORKDIR /build

COPY ./sec_store ./sec_store
COPY ./bot ./bot

RUN apk add pkgconfig openssl musl-dev libressl-dev
RUN cd bot && \
    cargo fetch --verbose && \
    cargo build --verbose --offline --release

FROM --platform=$BUILDPLATFORM alpine:3.18.4 as final_image

COPY \
    --from=build_stage \
    /build/bot/target/release/bot /usr/bin/

WORKDIR /app
VOLUME [ "/app" ]

CMD ["bot"]