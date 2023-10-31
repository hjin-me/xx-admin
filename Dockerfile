FROM rust:latest as builder
RUN apt-get update && apt-get install -y librust-clang-sys-dev
WORKDIR /build
COPY . .
RUN cargo build --release

FROM hjin/chrome:202310310152077a6aa9
COPY --from=builder /build/target/release/xx-admin /usr/local/bin/xx-admin
ENV CHROME /usr/bin/google-chrome
ENTRYPOINT ["xx-admin"]
