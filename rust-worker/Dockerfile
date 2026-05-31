#FROM rust:1.91.1-slim as build
#WORKDIR /project
#RUN apt-get update && apt-get install -y \
#    pkg-config \
#    libssl-dev \
#    && rm -rf /var/lib/apt/lists/*
#COPY ./Cargo.toml ./Cargo.lock ./
#COPY ./src ./src
#RUN cargo build --release
#
#FROM rust:1.91.1-slim AS final
#COPY --from=build /project/target/release/rust_worker /bin/rust_worker
#
#EXPOSE 8000
#CMD ["/bin/rust_worker"]

FROM lukemathwalker/cargo-chef:latest-rust-1.91.0 AS chef
WORKDIR /project

FROM chef AS planner
COPY ./Cargo.toml ./Cargo.lock ./
COPY ./src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update && apt-get install -y \
	pkg-config \
	libssl-dev \
	&& rm -rf /var/lib/apt/lists/*
COPY --from=planner /project/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY ./Cargo.toml ./Cargo.lock ./
COPY ./src ./src
RUN cargo build --release

FROM debian:trixie-slim AS final
RUN apt-get update && apt-get install -y \
	pkg-config \
	libssl-dev \
	&& rm -rf /var/lib/apt/lists/*
COPY --from=builder /project/target/release/rust_worker /bin/rust_worker
EXPOSE 8000
CMD ["/bin/rust_worker"]
