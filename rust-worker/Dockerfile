#FROM rust:1.91.1-slim as build
#WORKDIR /project
#COPY ./Cargo.toml ./Cargo.lock ./
#COPY ./src ./src
#RUN cargo build --release

#FROM rust:1.91.1-slim AS final
#COPY --from=build /project/target/release/rust-worker /bin/rust-worker

#EXPOSE 8000
#CMD ["/bin/rust-worker"]

FROM rust:1.91.1-slim
#WORKDIR /project
COPY dummy.rs ./
COPY Cargo.toml ./ 
COPY Cargo.lock ./
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY . .
RUN cargo build --release
EXPOSE 8000
CMD ["target/release/rust-worker"]
