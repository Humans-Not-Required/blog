# Stage 1: Build frontend
FROM node:20-slim AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install
COPY frontend/ ./
RUN npm run build

# Stage 2: Build backend
FROM rust:1-slim-bookworm AS backend
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release 2>/dev/null || true
COPY backend/src ./src
COPY backend/openapi.json ./openapi.json
RUN touch src/**/*.rs src/*.rs 2>/dev/null || true && cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend /app/target/release/blog ./blog
COPY --from=frontend /app/frontend/dist ./static
RUN mkdir -p data
ENV ROCKET_PORT=3004
ENV ROCKET_ADDRESS=0.0.0.0
ENV DATABASE_PATH=/app/data/blog.db
ENV STATIC_DIR=/app/static
EXPOSE 3004
CMD ["./blog"]
