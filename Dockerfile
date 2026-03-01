FROM rust:1.93-bookworm AS builder
WORKDIR /usr/src/app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /usr/src/app/target/release/class-chat-bot .
EXPOSE 8080
CMD ["./class-chat-bot"]
