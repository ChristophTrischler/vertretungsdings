FROM rust:latest AS chef 
RUN cargo install cargo-chef
RUN dpkg --add-architecture armhf
RUN apt-get update && apt-get install -y  crossbuild-essential-armhf
ENV PKG_CONFIG_SYSROOT_DIR "/usr/lib/arm-linux-gnueabihf/" 
RUN rustup target add armv7-unknown-linux-gnueabihf
WORKDIR /vertretungsapi
COPY . . 

FROM chef AS planner
COPY . .
WORKDIR /vertretungsapi
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /vertretungsapi 
COPY --from=planner /vertretungsapi/recipe.json recipe.json
RUN cargo chef cook --release  --target armv7-unknown-linux-gnueabihf --recipe-path recipe.json
COPY . .
RUN cargo build --release --target armv7-unknown-linux-gnueabihf --bin vertretungsapi

FROM --platform=linux/arm32 arm32v7/debian:latest as runtime
WORKDIR /vertretungsapi
ENV RUST_LOG "info, lopdf=off"
COPY --from=builder /vertretungsapi/target/armv7-unknown-linux-gnueabihf/release/vertretungsapi /usr/bin/
CMD [ "/usr/bin/vertretungsapi" ] 
