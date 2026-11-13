# syntax=docker/dockerfile:1
# Image cho backend. Một lần build ra cả hai binary, mỗi binary một image:
#   docker build --target api    -t sls-api    backend
#   docker build --target worker -t sls-worker backend
# Thường không gọi tay mà để deploy/docker-compose.yml build.

FROM rust:1.99.1-slim-trixie AS build
WORKDIR /src
COPY . .
# Cache thư viện tải về và thư mục target giữa các lần build → build lại chỉ dịch phần code đổi.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p sls-api -p sls-worker \
 && mkdir -p /out \
 && cp target/release/sls-api target/release/sls-worker /out/

# Distroless: chỉ có thư viện C + chứng chỉ CA, không có shell; chạy bằng user không phải root.
FROM gcr.io/distroless/cc-debian13:nonroot AS api
COPY --from=build /out/sls-api /usr/local/bin/sls-api
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/sls-api"]

FROM gcr.io/distroless/cc-debian13:nonroot AS worker
COPY --from=build /out/sls-worker /usr/local/bin/sls-worker
ENTRYPOINT ["/usr/local/bin/sls-worker"]
