FROM docker.io/lukemathwalker/cargo-chef:0.1.63-rust-1.76.0-bookworm as chef
LABEL stage=build

RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y clang libssl-dev lld pkg-config \
 && groupadd -g 1000 app \
 && useradd -u 1000 -g 1000 -s /bin/bash -M app \
 && mkdir /app \
 && chown app:app /app

FROM chef AS planner
LABEL stage=build

USER app

WORKDIR /app
COPY --chown=app:app . .
RUN cargo chef prepare --recipe-path recipe.json

FROM planner AS builder
LABEL stage=build

ENV SQLX_OFFLINE true
USER app

COPY --from=planner --chown=app:app /app/recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json

COPY --chown=app:app . .
RUN cargo build --release

FROM docker.io/library/debian:bookworm-slim AS runtime

RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y ca-certificates libssl3 \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd -g 1000 app \
 && useradd -u 1000 -g 1000 -s /bin/bash -M app

ENV APP_ENVIRONMENT production
USER app

WORKDIR /app
COPY --chown=root:root --chmod=444 configuration ./configuration
COPY --from=builder --chown=root:root --chmod=555 /app/target/release/zero2prod .

ENTRYPOINT ["./zero2prod"]
