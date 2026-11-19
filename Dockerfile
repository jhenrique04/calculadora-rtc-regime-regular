FROM rust:1.99-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

COPY . .

RUN cargo build --release --locked -p rtc-server -p rtc-banco \
 && ./target/release/rtc-gerar-banco --sql flyway/sql --banco calculadora/db/calculadora-pro.db

FROM alpine:latest

RUN apk update && apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/target/release/rtc-server ./rtc-server
COPY --from=builder /app/calculadora/db ./calculadora/db

EXPOSE 8080 9101

ENTRYPOINT ["./rtc-server"]
