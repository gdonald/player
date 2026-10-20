# The image the home lab's Jenkins runs test.sh in. It carries what the GitHub
# workflow installs step by step: stable and nightly Rust with the wasm32
# target and llvm-tools, cargo-llvm-cov, trunk, Node 22, Playwright's system
# libraries, the Postgres client tools test.sh resets databases with, and clang,
# which minicov uses to compile its coverage runtime for wasm32.
# Playwright's Chromium itself is downloaded by the Jenkinsfile, so it always
# matches the version in features/package-lock.json.
#
# Build and push from the repository root:
#   docker buildx build --platform linux/amd64 -t harbor.home.gregdonald.com/player/ci:latest --push ci
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
       build-essential \
       ca-certificates \
       clang \
       curl \
       git \
       jq \
       pkg-config \
       postgresql-client \
  && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
  && apt-get install -y --no-install-recommends nodejs \
  && rm -rf /var/lib/apt/lists/*

ARG PLAYWRIGHT_VERSION=1.63.0
RUN npx -y "playwright@${PLAYWRIGHT_VERSION}" install-deps chromium \
  && rm -rf /var/lib/apt/lists/* /root/.npm

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# test.sh expects stable as the default and calls nightly as +nightly.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable \
  && rustup component add rustfmt clippy llvm-tools-preview \
  && rustup target add wasm32-unknown-unknown \
  && rustup toolchain install nightly --profile minimal --component llvm-tools-preview --target wasm32-unknown-unknown \
  && cargo install cargo-llvm-cov --version 0.9.1 --locked \
  && cargo install trunk --version 0.21.14 --locked \
  && rm -rf /usr/local/cargo/registry /usr/local/cargo/git \
  && chmod -R a+rX /usr/local/rustup /usr/local/cargo

# Jenkins runs this image as uid 1000, which is the image's `ubuntu` user, so
# test.sh's `whoami` has a name to print.
