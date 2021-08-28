# build stage
FROM rust as build

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

# final stage, no rust, only the compiled binary
FROM debian:buster-slim

COPY --from=build /medusa/target/release/medusa .

CMD ["./medusa", "--services", "/etc/medusa/services.d", "--records", "/var/lib/medusa/records"]