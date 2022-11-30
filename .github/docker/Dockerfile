FROM paritytech/ci-linux:production

WORKDIR /var/www/faterium-node

ENV CARGO_HOME=/var/www/faterium-node/.cargo

COPY node node
COPY pallets pallets
COPY runtime runtime
COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml

RUN cargo build --release

ENTRYPOINT ["./target/release/faterium-node"]
