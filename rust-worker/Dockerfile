FROM rust:1.91.1-slim as build
WORKDIR /project
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*
COPY ./Cargo.toml ./Cargo.lock ./
COPY ./src ./src
RUN cargo build --release

FROM rust:1.91.1-slim AS final
COPY --from=build /project/target/release/rust_worker /bin/rust_worker

EXPOSE 8000
CMD ["/bin/rust_worker"]

