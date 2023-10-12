FROM --platform=$BUILDPLATFORM rust:1.73 as build_stage

WORKDIR /build

COPY ./bot ./bot
COPY ./sec_store ./sec_store

RUN cd bot && \
    cargo fetch --verbose && \
    cargo build --verbose --offline --release

FROM --platform=$BUILDPLATFORM debian:bullseye-slim as final_image
RUN apt-get update \
    && apt-get install ca-certificates -y

COPY \
    --from=build_stage \
    /build/bot/target/release/bot /usr/bin/

WORKDIR /app
VOLUME [ "/app" ]

CMD ["bot"]