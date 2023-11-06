FROM rust:latest as builder
RUN apt-get update && apt-get install -y librust-clang-sys-dev
WORKDIR /build
COPY . .
RUN cargo build --release

FROM hjin/chrome:2023110603475551c7e6
COPY --from=builder /build/target/release/xx-admin /usr/local/bin/xx-admin
ENV CHROME /usr/bin/google-chrome
ENTRYPOINT ["xx-admin"]
