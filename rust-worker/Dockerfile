FROM rust:1.91.1-slim as build
WORKDIR /project
COPY ./Cargo.toml ./Cargo.lock ./
COPY ./src ./src
RUN cargo build --locked --release

FROM rust:1.91.1-slim AS final
COPY --from=build /project/target/release/rust-worker /bin/rust-worker

EXPOSE 8000
CMD ["/bin/rust-worker"]
