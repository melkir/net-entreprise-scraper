
FROM rust:1.61 as build

RUN USER=root cargo new --bin scraper
WORKDIR /usr/src/app

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

RUN rm src/*.rs
COPY ./src ./src

RUN rm ./target/release/deps/scraper*
RUN cargo build --release

FROM debian:buster-slim
COPY --from=build /usr/src/app/target/release/scraper .

CMD ["./scraper"]
