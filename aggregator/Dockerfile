FROM rust:1.69-bullseye as base
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
RUN apt-get update \
    && apt-get -y upgrade \
    && apt-get install -y wget \
    && mkdir /tmp/chrome \
    && wget --no-verbose -O /tmp/chrome/chrome.deb https://dl.google.com/linux/chrome/deb/pool/main/g/google-chrome-stable/google-chrome-stable_113.0.5672.92-1_amd64.deb \
    && apt-get install -y /tmp/chrome/chrome.deb \
    && wget --no-verbose -O /tmp/chrome/chromedriver.zip https://chromedriver.storage.googleapis.com/113.0.5672.63/chromedriver_linux64.zip \
    && unzip /tmp/chrome/chromedriver.zip -d /tmp/chrome \
    && chmod +x /tmp/chrome/chromedriver \
    && mv /tmp/chrome/chromedriver /usr/local/bin \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /tmp/chrome
CMD ["/bin/bash", "-c", "chromedriver & cargo test -- --show-output"]

FROM base as build
RUN cargo build --release

FROM debian:bullseye-slim
WORKDIR /usr/app
RUN apt-get update \
    && apt-get -y upgrade \
    && apt-get install -y wget unzip \
    && mkdir /tmp/chrome \
    && wget --no-verbose -O /tmp/chrome/chrome.deb https://dl.google.com/linux/chrome/deb/pool/main/g/google-chrome-stable/google-chrome-stable_113.0.5672.92-1_amd64.deb \
    && apt-get install -y /tmp/chrome/chrome.deb \
    && wget --no-verbose -O /tmp/chrome/chromedriver.zip https://chromedriver.storage.googleapis.com/113.0.5672.63/chromedriver_linux64.zip \
    && unzip /tmp/chrome/chromedriver.zip -d /tmp/chrome \
    && chmod +x /tmp/chrome/chromedriver \
    && mv /tmp/chrome/chromedriver /usr/local/bin \
    && rm -rf /var/lib/apt/lists/* \
    && rm -rf /tmp/chrome
COPY --from=build /usr/src/oshirase/aggregator/target/release/aggregator /usr/local/bin/aggregator
COPY --from=build /usr/src/oshirase/aggregator/config/config.toml ./config/config.toml
COPY --from=build /usr/src/oshirase/aggregator/graphql/ ./graphql/
CMD ["/bin/bash", "-c", "chromedriver & aggregator"]
