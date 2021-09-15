FROM rust:1.54 as planner
WORKDIR jump-diffusion
# We only pay the installation cost once, 
# it will be cached from the second build onwards
# To ensure a reproducible build consider pinning 
# the cargo-chef version with `--version X.X.X`
RUN cargo install cargo-chef 

COPY . .

RUN cargo chef prepare  --recipe-path recipe.json

FROM rust:1.54 as cacher
WORKDIR jump-diffusion
RUN cargo install cargo-chef
COPY --from=planner /jump-diffusion/recipe.json recipe.json
RUN --mount=type=ssh cargo chef cook --release --recipe-path recipe.json

FROM rust:1.54 as builder
WORKDIR jump-diffusion
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /jump-diffusion/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN --mount=type=ssh cargo build --release --bin jump-diffusion

FROM debian:buster-slim as runtime
WORKDIR jump-diffusion
COPY --from=builder /jump-diffusion/target/release/jump-diffusion /usr/local/bin
RUN apt-get update && apt-get -y install ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["/usr/local/bin/jump-diffusion"]
