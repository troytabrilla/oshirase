FROM rust:1.69 as base
WORKDIR /usr/src/oshirase/aggregator
COPY ./Cargo.toml .
COPY ./Cargo.lock .
RUN mkdir ./src && echo 'fn main() {}' > ./src/main.rs
RUN cargo build
RUN cargo build --release
RUN rm -rf ./src
COPY ./src ./src
COPY ./graphql ./graphql
COPY ./config/config.docker.toml ./config/config.toml
RUN touch -a -m ./src/main.rs

FROM base as test
RUN cargo test

FROM base as build
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=build /usr/src/oshirase/aggregator/target/release/aggregator /usr/local/bin/aggregator
COPY --from=build /usr/src/oshirase/aggregator/config/config.toml ./config/config.toml
COPY --from=build /usr/src/oshirase/aggregator/graphql/ ./graphql/
CMD ["aggregator", "config/config.toml"]
