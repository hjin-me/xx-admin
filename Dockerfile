FROM hjin/leptos-build:2023072001413511edb3 as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM hjin/chrome:202310310152077a6aa9
COPY --from=builder /build/target/release/xx-admin /usr/local/bin/xx-admin
ENV CHROME /usr/bin/google-chrome
ENTRYPOINT ["xx-admin"]
