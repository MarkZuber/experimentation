FROM rust:1.88 AS builder

WORKDIR /app

# Install protoc
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.toml
COPY proto/ proto/
COPY db-service/ db-service/
COPY cache-service/ cache-service/
COPY product-backend/ product-backend/

# Build only db-service
RUN cargo build --release -p db-service

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/db-service /usr/local/bin/db-service

EXPOSE 50051
CMD ["db-service"]
