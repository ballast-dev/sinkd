FROM nixos/nix:latest

# Update nix channels
RUN nix-channel --update

# Install development tools and dependencies
RUN nix-env -iA \
    nixpkgs.rustup \
    nixpkgs.just \
    nixpkgs.zip \
    nixpkgs.unzip \
    nixpkgs.gnutar \
    nixpkgs.gzip \
    nixpkgs.git \
    nixpkgs.glibc \
    nixpkgs.glibc.dev \
    nixpkgs.pkgsCross.mingwW64.stdenv.cc \
    nixpkgs.mosquitto \
    nixpkgs.openssl \
    nixpkgs.openssl.dev \
    nixpkgs.musl \
    nixpkgs.musl.dev \
    nixpkgs.llvm \
    nixpkgs.clang \
    nixpkgs.cmake \
    nixpkgs.pkg-config \
    nixpkgs.perl \
    nixpkgs.python3

# Set up Rust toolchain and targets
RUN rustup default stable \
    && rustup target add x86_64-pc-windows-gnu \
    && rustup target add x86_64-unknown-linux-musl \
    && rustup target add aarch64-unknown-linux-musl \
    && rustup target add x86_64-apple-darwin \
    && rustup target add aarch64-apple-darwin

# Set up cross-compilation environment variables
ENV PATH="/root/.nix-profile/bin:$PATH"
ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="x86_64-w64-mingw32-gcc"
ENV OPENSSL_STATIC=1
ENV OPENSSL_VENDORED=1
ENV CC_x86_64_unknown_linux_musl="musl-gcc"
ENV CC_aarch64_unknown_linux_musl="musl-gcc"
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV RUST_BACKTRACE=1
ENV CARGO_INCREMENTAL=1
ENV CARGO_TARGET_DIR=./target

# Create a non-root user for security
RUN adduser -D -s /bin/sh sinkd

# Create workspace directory
WORKDIR /sinkd
RUN chown sinkd:sinkd /sinkd

# Switch to non-root user
USER sinkd

LABEL org.opencontainers.image.title="sinkd Cross-Compilation Environment"
LABEL org.opencontainers.image.description="Complete Rust cross-compilation environment for sinkd"
LABEL org.opencontainers.image.source="https://github.com/ballast-dev/sinkd"
LABEL org.opencontainers.image.vendor="Ballast Development"

# Default command - start a shell with environment ready
CMD ["/bin/sh", "-c", "echo '🚀 sinkd Cross-Compilation Environment Ready!' && /bin/sh"]