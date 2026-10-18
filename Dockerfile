# Multi-stage build: the Rust toolchain stays in the builder stage;
# the published image is a slim Debian runtime carrying only the
# stripped binary and the static assets. (The previous single-stage
# build ran the app from the full rust:latest toolchain image,
# which put the image over 300MB.)

FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . /app
RUN cargo build --release && strip target/release/brunnylol

FROM debian:bookworm-slim
# ca-certificates: the AI feature makes outbound HTTPS calls.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/brunnylol /usr/local/bin/brunnylol
# static/ must come from the builder stage, not the build context:
# build.rs generates the favicon PNGs into it during the build, and
# those files are gitignored so a fresh checkout doesn't have them.
COPY --from=builder /app/static /app/static
ENV BRUNNYLOL_PORT=8000
ENV BRUNNYLOL_DB=/data/brunnylol.db
EXPOSE 8000
CMD ["brunnylol"]
