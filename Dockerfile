# build stage
FROM rust as build

# install native dependencies
RUN apt-get update && apt-get install -y openssl libpcap0.8-dev

# create a new empty shell project
RUN USER=root cargo new --bin medusa
WORKDIR /medusa

# copy contents and cache dependencies
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs
COPY ./src ./src

# build
RUN rm ./target/release/deps/medusa*
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y libpcap0.8
COPY --from=build /medusa/target/release/medusa /usr/bin/medusa
ENTRYPOINT ["/usr/bin/medusa", "--services", "/etc/medusa/services.d", "--records", "/var/lib/medusa/records"]
