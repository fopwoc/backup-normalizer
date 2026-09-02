FROM rust:1.97-alpine AS builder

RUN apk add --no-cache musl-dev
WORKDIR /source
COPY . .

ARG BUILD_VERSION=development
RUN BUILD_VERSION="$BUILD_VERSION" cargo build --locked --release --package backup-normalizer

FROM scratch

COPY --from=builder /source/target/release/backup-normalizer /backup-normalizer
USER 65532:65532
ENTRYPOINT ["/backup-normalizer"]

