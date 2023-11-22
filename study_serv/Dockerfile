FROM hjin/dioxus-build:202311210842339af88f as builder
WORKDIR /build
COPY . .
#RUN echo $(git rev-parse HEAD) > commit_id.txt
RUN cd study_serv \
    && dx build --release --features web \
    && cargo build --release --features ssr

FROM hjin/chrome:20231106065551772b5a
COPY --from=builder /build/target/release/study_serv /usr/local/bin/study_serv
COPY --from=builder /build/study_serv/dist /webser/www/dist
ENV CHROME /usr/bin/google-chrome
ENV RUST_LOG warn,study_serv=info,study_core=info,study=info
WORKDIR /webser/www
ENTRYPOINT ["study_serv"]

