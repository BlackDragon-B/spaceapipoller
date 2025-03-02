ARG BUILD_FROM
FROM rust:1.85-alpine as builder

WORKDIR /app

COPY . .

RUN apk --update add openssl openssl-dev musl-dev openssl-libs-static

RUN cargo build --release

FROM $BUILD_FROM

WORKDIR /app

COPY --from=builder /app/target/release/spaceapipoller /app/spaceapipoller

ENTRYPOINT ["./spaceapipoller"]