FROM rust:latest AS builder

RUN update-ca-certificates

WORKDIR /app

COPY ./ .
COPY rest-catalog-open-api.yaml rest-catalog-open-api.yaml

# Install cmake
RUN apt-get update && apt-get install -y cmake

RUN cargo build --release

####################################################################################################
## Final image
####################################################################################################

FROM debian:bookworm-slim

# Copy the binary from the builder stage
WORKDIR /app

RUN apt-get update && apt-get install -y openssl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/bucketd ./
COPY --from=builder /app/rest-catalog-open-api.yaml rest-catalog-open-api.yaml

CMD ["./bucketd"]
