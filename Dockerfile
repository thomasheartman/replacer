FROM rust:1.48 as build

ENV target=x86_64-unknown-linux-musl

WORKDIR /build

RUN rustup target add $target

COPY . .

RUN cargo install --target $target --path .

FROM scratch

COPY --from=build /usr/local/cargo/bin/replacer .

ENTRYPOINT ["./replacer"]
CMD ["-h"]
