FROM rust:1.48 as build

WORKDIR /build

COPY . .

RUN cargo install --path .

FROM debian:buster-slim

COPY --from=build /build/target/release/replacer /usr/local/bin/replacer

ENTRYPOINT ["replacer"]
CMD ["-h"]
