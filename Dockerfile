FROM rust:1.89-slim as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

FROM archlinux:base

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/app/target/release/net-entreprise-scraper .

EXPOSE 8000

CMD ["./net-entreprise-scraper"] 