FROM rust:latest as builder
RUN apt-get update && apt-get install -y librust-clang-sys-dev
WORKDIR /build
COPY . .
RUN cd admin_serv && cargo build --release

FROM hjin/chrome:20231106065551772b5a
COPY --from=builder /build/target/release/admin_serv /usr/local/bin/admin_serv
ENV CHROME /usr/bin/google-chrome
ENV RUST_LOG warn,admin_serv=info,wx=info,study=info,study_core=info,
ENTRYPOINT ["admin_serv"]
