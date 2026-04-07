default:
    @just --list

# Build for Linux (requires nix dev shell or GTK4 libs)
build:
    cargo build --release

# Build for Windows (cross-compile from Linux)
build-windows:
    nix-shell -p pkgsCross.mingwW64.stdenv.cc -p pkgsCross.mingwW64.windows.pthreads --run "CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc cargo build --release --target x86_64-pc-windows-gnu"

# Build via nix
build-nix:
    nix build

# Check both targets compile
check:
    cargo check
    nix-shell -p pkgsCross.mingwW64.stdenv.cc -p pkgsCross.mingwW64.windows.pthreads --run "CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc cargo check --target x86_64-pc-windows-gnu"

# Run locally
run:
    cargo run

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# Lint
clippy:
    cargo clippy -- -W clippy::all

# Run tests
test:
    cargo test

# Format + lint + test
ci: fmt-check clippy test check
