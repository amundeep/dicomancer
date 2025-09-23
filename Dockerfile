FROM rust:1.82-slim

# Install build deps commonly needed by crates
RUN apt-get update && apt-get install -y \
    libx11-dev libxext-dev libxrandr-dev libxi-dev \
    libxcursor-dev libxinerama-dev libgl1-mesa-dev \
    libxkbcommon-x11-0 fonts-dejavu fontconfig \
    pkg-config libssl-dev build-essential clang cmake git vim \
    && rm -rf /var/lib/apt/lists/*

# Install rust components only
RUN rustup component add clippy rustfmt

# Set working directory inside container
WORKDIR /app

# Cache deps first (only works if Cargo.toml/Cargo.lock exist)
# This speeds up rebuilds since dependencies get cached separately
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch || true

# Copy the rest of the source code
COPY . .

# Default command (optional)
CMD ["cargo", "build", "--release"]