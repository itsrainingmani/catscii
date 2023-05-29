# syntax = docker/dockerfile:1.4

################################################################################
FROM ubuntu:20.04 AS base

################################################################################
FROM base AS builder

# Install compile-time dependencies
RUN set -eux; \
		apt update; \
		apt install -y --no-install-recommends \
		openssh-client git-core curl ca-certificates gcc libc6-dev pkg-config libssl-dev \
		libsqlite3-dev \
		;

# Install rustup
RUN --mount=type=cache,target=/root/.rustup \
		set -eux; \
		curl --location --fail \
		"https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init" \
		--output rustup-init; \
		chmod +x rustup-init; \
		./rustup-init -y --no-modify-path --default-toolchain stable; \
		rm rustup-init;

# Add rustup to path, check that it works
ENV PATH=${PATH}:/root/.cargo/bin
RUN set -eux; \
		rustup --version;

# Copy sources and build them
WORKDIR /app
COPY src src
COPY .cargo .cargo
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
RUN mkdir -p ~/.ssh/ && ssh-keyscan ssh.shipyard.rs >> ~/.ssh/known_hosts
RUN --mount=type=cache,target=/root/.rustup \
		--mount=type=cache,target=/root/.cargo/registry \
		--mount=type=cache,target=/root/.cargo/git \
		--mount=type=cache,target=/app/target \
		--mount=type=ssh \
		--mount=type=secret,id=shipyard-token \
		set -eux; \
		export CARGO_REGISTRIES_ITSRAININGMANI_TOKEN=$(cat /run/secrets/shipyard-token); \
		rustc --version; \
		cargo build --release; \
		objcopy --compress-debug-sections ./target/release/catscii ./catscii

################################################################################
FROM base AS app

SHELL ["/bin/bash", "-c"]

# Install run-time dependencies, remove extra APT files afterwards.
# This must be done in the same `RUN` command, otherwise it doesn't help
# to reduce the image size.
RUN set -eux; \
		apt update; \
		apt install -y --no-install-recommends \
		ca-certificates \
		libsqlite3-0 \
		; \
		apt clean autoclean; \
		apt autoremove --yes; \
		rm -rf /var/lib/{apt,dpkg,cache,log}/

# Copy app from builder
WORKDIR /app
COPY --from=builder /app/catscii .

RUN mkdir /db 
COPY ./db/GeoLite2-Country.mmdb /db/

CMD ["/app/catscii"]