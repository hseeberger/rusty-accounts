ARG RUST_VERSION=1.76.0

FROM rust:$RUST_VERSION-bookworm AS builder
ARG PROFILE=release
WORKDIR /build
COPY . .
RUN \
  --mount=type=cache,target=/build/target/ \
  --mount=type=cache,target=/usr/local/cargo/registry/ \
  cargo build --profile $PROFILE && \
  dir=release && if [ $PROFILE = dev ]; then dir=debug; fi && \
  cp ./target/$dir/rusty-accounts /

FROM debian:bookworm-slim AS final
RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/nonexistent" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "10001" \
  appuser
COPY --from=builder /rusty-accounts /usr/local/bin
RUN chown appuser /usr/local/bin/rusty-accounts
COPY --from=builder /build/config /opt/rusty-accounts/config
RUN chown -R appuser /opt/rusty-accounts
USER appuser
ENV RUST_LOG="rusty_bank=debug,info"
WORKDIR /opt/rusty-accounts
ENTRYPOINT ["rusty-accounts"]
EXPOSE 8080/tcp
