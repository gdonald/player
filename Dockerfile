FROM rust:1.96-trixie AS build

ARG TRUNK_VERSION=0.21.14

RUN rustup target add wasm32-unknown-unknown \
    && curl -sSfL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-$(uname -m)-unknown-linux-gnu.tar.gz" \
    | tar xz -C /usr/local/bin trunk

WORKDIR /src
COPY . .

RUN cargo build --release --locked -p player-server --bin player

RUN cd crates/client && trunk build --release --locked

FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 1000 --no-create-home player

COPY --from=build /src/target/release/player /usr/local/bin/player
COPY --from=build /src/crates/client/dist /app/dist

ENV WEB_ROOT=/app/dist \
    PORT=3000

USER player
EXPOSE 3000

ENTRYPOINT ["player"]
CMD ["serve"]
