FROM --platform=linux/amd64 lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /vertretungsapi

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /vertretungsapi/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin vertretungsapi

FROM --platform=linux/amd64 debian:stable-slim AS runtime
WORKDIR /vertretungsapi
RUN apt-get update && \ 
    apt-get install ca-certificates -y && \
    apt-get clean && \
    update-ca-certificates
COPY --from=builder /vertretungsapi/target/release/vertretungsapi /usr/bin/
CMD [ "/usr/bin/vertretungsapi" ] 