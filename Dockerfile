FROM alpine:latest AS builder

WORKDIR /app

RUN apk update
RUN apk upgrade
RUN apk add curl libgcc gcc libc-dev
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable --profile minimal

COPY src ./src/
COPY Cargo.toml .

RUN source $HOME/.cargo/env && cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest

WORKDIR /app

RUN apk update
RUN apk upgrade
RUN apk add ca-certificates

COPY commands.json .
COPY config.json .
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/trangarbot .

CMD ["./trangarbot"]

