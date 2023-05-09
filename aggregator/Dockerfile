FROM rust:1.69 as builder
WORKDIR /usr/src/oshirase/aggregator
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/aggregator /usr/local/bin/aggregator
COPY ./config/config.docker.toml ./config/config.toml
ADD ./graphql/ ./graphql/
CMD ["aggregator"]
