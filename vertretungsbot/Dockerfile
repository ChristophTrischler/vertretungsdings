FROM rust:latest AS chef 
RUN cargo install cargo-chef
RUN dpkg --add-architecture armhf
RUN apt-get update && apt-get install -y openssl:armhf libssl-dev:armhf crossbuild-essential-armhf
ENV PKG_CONFIG_SYSROOT_DIR "/usr/lib/arm-linux-gnueabihf/" 
RUN rustup target add armv7-unknown-linux-gnueabihf
WORKDIR /vertretungsbot
COPY . . 

FROM chef AS planner
WORKDIR /vertretungsbot
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /vertretungsbot
COPY --from=planner /vertretungsbot/recipe.json recipe.json
RUN cargo chef cook --release --target armv7-unknown-linux-gnueabihf --recipe-path recipe.json
COPY . .
RUN cargo build --release --target armv7-unknown-linux-gnueabihf --bin vertretungsbot

FROM --platform=linux/arm32 arm32v7/debian:latest as runtime
WORKDIR /vertretungsbot
RUN apt-get update && \ 
    apt-get install ca-certificates -y && \
    apt-get clean && \
    update-ca-certificates
COPY --from=builder /vertretungsbot/target/armv7-unknown-linux-gnueabihf/release/vertretungsbot /usr/bin/
CMD [ "/usr/bin/vertretungsbot" ] 