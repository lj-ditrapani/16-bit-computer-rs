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

## Formatting

```bash
cargo fmt
```

## Running

```bash
cargo run --release -- --cartridge <path-to-cartridge-file>
```

## Status

Early development - CPU and memory subsystems in progress.
