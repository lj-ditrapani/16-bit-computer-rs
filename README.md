# LJD 16-bit Computer Emulator

A Rust implementation of the LJD 16-bit computer emulator.

## Overview

This project implements an emulator for the LJD 16-bit computer, a cartridge-based 16-bit computer system with video display, audio, and gamepad input capabilities.

## Specification

See the [16-bit-computer-specification](https://github.com/lj-ditrapani/16-bit-computer-specification) repository for the complete hardware specification.

## Architecture

See [architecture.md](architecture.md) for detailed architecture documentation of this emulator implementation.

## Building

```bash
cargo build --release
```

## Testing

Run all tests:

```bash
cargo test
```

Run a specific test:

```bash
cargo test test_name
```

## Formatting

```bash
cargo fmt
```

## Running

```bash
cargo run -- --cartridge <path-to-cartridge-file>
```

## Release

Create a release build:

```bash
cargo build --release
```

The release binary will be located at `target/release/ljd-16-bit-computer-rs`.

For cross-platform releases, use [cargo-cross](https://github.com/cross-rs/cross):

```bash
# Install cross
cargo install cross --git https://github.com/cross-rs/cross

# Build for a specific target
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target x86_64-pc-windows-gnu
cross build --release --target x86_64-apple-darwin
```

## Status

Early development - CPU and memory subsystems in progress.
