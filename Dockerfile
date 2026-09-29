# Linux build and test image, for developing on Linux (not for shipping).
# The same packages as the linux-app job in .github/workflows/ci.yml. See "Developing on Linux"
# in README.md, and AGENTS.md for the checks.
#
#   docker build -t yahaha-linux .
#   docker run --rm yahaha-linux                  # the Linux checks on the copied source
#   docker run --rm -v "$PWD":/work -v /work/app/node_modules yahaha-linux
#                                                 # the same on the live checkout
#   docker run --rm yahaha-linux cargo test --profile test-quick <name>  # any one command
FROM node:22-bookworm-slim AS node

FROM rust:1-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libasound2-dev \
        build-essential \
        libwebkit2gtk-4.1-dev \
        libgtk-3-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        libsoup-3.0-dev \
        libjavascriptcoregtk-4.1-dev \
    && rm -rf /var/lib/apt/lists/*

# Node 22 from the official image (same Debian release), without a third-party apt repo.
COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx \
    && node --version && npm --version

WORKDIR /work
# Linux artifacts stay apart from a mounted macOS checkout's target/.
ENV CARGO_TARGET_DIR=/work/target/linux

# The app's node_modules are Linux-native (esbuild, rollup), so they live in the image.
# Mounting the checkout over /work with `-v /work/app/node_modules` keeps these.
COPY app/package.json app/package-lock.json app/
RUN cd app && npm ci

COPY . .

# The Linux checks from AGENTS.md, in order, stopping at the first failure.
CMD ["bash", "-ec", "cargo check --no-default-features --lib; cargo test --profile test-quick; (cd app/src-tauri && cargo test); (cd app && npm run verify)"]
