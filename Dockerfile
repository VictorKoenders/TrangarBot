FROM alpine:latest AS builder

WORKDIR /app

RUN apk update
RUN apk upgrade
RUN apk add curl libgcc gcc libc-dev
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable --profile minimal

# load and pre-compile the cargo crates
COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir src && \
    echo "fn main(){}" > src/main.rs
RUN source $HOME/.cargo/env && cargo build --release --target x86_64-unknown-linux-musl

# Copy in the source code
COPY src ./src/

# Make sure the correct src/main.rs is newer
RUN touch src/main.rs

RUN source $HOME/.cargo/env && cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest

WORKDIR /app

RUN apk update
RUN apk upgrade
RUN apk add ca-certificates

COPY config.json .
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/trangarbot .

CMD ["./trangarbot"]

