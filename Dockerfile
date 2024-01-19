FROM messense/rust-musl-cross:x86_64-musl as builder
RUN rustup update && \
    rustup target add x86_64-unknown-linux-musl
#RUN apt-get update && apt-get install -y musl-tools libssl-dev
#ENV TARGET_CC=x86_64-linux-musl-gcc
#ENV RUSTFLAGS="-C linker=x86_64-linux-musl-gcc"
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release --target=x86_64-unknown-linux-musl

FROM alpine:latest
WORKDIR /app
COPY statics ./statics
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/web_large_file_uploader .
CMD ["./web_large_file_uploader"]