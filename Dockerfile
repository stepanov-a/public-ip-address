FROM rust:latest as build
WORKDIR /app
COPY . .
RUN cargo build -p ip-service --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=build /app/target/release/ip-service /usr/local/bin/ip-service
EXPOSE 8080
CMD ["ip-service"]
