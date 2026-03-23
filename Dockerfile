# ============================================================
# Stage 1: Build (chef + compile)
# ============================================================
FROM rust:1.88-bookworm AS chef
RUN cargo install cargo-chef --version 0.1.71 --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies only (cached unless Cargo.lock changes)
# Use x86-64-v3 for broad AVX2 compatibility in modern cloud environments
ENV RUSTFLAGS="-C target-cpu=x86-64-v3"
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin vrc-backend && \
    strip /app/target/release/vrc-backend

# ============================================================
# Stage 2: Runtime (minimal)
# ============================================================
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1000 app && \
    useradd --uid 1000 --gid app --no-create-home app

COPY --from=builder /app/target/release/vrc-backend /usr/local/bin/vrc-backend
COPY --from=builder /app/vrc-backend/migrations /app/migrations

USER app
WORKDIR /app
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["curl", "--fail", "--silent", "http://127.0.0.1:3000/health"]

ENTRYPOINT ["/usr/local/bin/vrc-backend"]
