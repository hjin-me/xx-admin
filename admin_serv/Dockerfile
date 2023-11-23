FROM hjin/dioxus-build:202311210842339af88f as builder
WORKDIR /build
COPY . .
RUN cd admin_serv \
    && dx build --release --features web \
    && cargo build --release --features ssr

FROM hjin/chrome:20231106065551772b5a
COPY --from=builder /build/target/release/admin_serv /usr/local/bin/admin_serv
COPY --from=builder /build/admin_serv/dist /webser/www/dist
ENV CHROME /usr/bin/google-chrome
ENV RUST_LOG warn,admin_serv=info,study_core=info,study=info
WORKDIR /webser/www
ENTRYPOINT ["admin_serv"]

