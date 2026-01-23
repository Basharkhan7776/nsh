FROM rust:latest

WORKDIR /app

# Install dependencies (protobuf needed for some crates if features enabled, but basic build seems fine)
RUN apt-get update && apt-get install -y pkg-config libssl-dev

COPY . .

# Pre-build to cache dependencies
RUN cargo build --release || true

CMD ["/bin/bash"]
