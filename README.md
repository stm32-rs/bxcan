# bxCAN peripheral driver

[![crates.io](https://img.shields.io/crates/v/bxcan.svg)](https://crates.io/crates/bxcan)
[![docs.rs](https://docs.rs/bxcan/badge.svg)](https://docs.rs/bxcan/)
![CI](https://github.com/stm32-rs/bxcan/workflows/CI/badge.svg)

This crate implements a driver for the bxCAN peripheral found in many low- to
middle-end STM32 microcontrollers.

Please refer to the [changelog](CHANGELOG.md) to see what changed in the last
releases.

## Usage

Add an entry to your `Cargo.toml`:

```toml
[dependencies]
bxcan = "0.5.1"
```

Check the [API Documentation](https://docs.rs/bxcan/) for how to use the
crate's functionality.

## Rust version support

This crate supports at least the 3 latest stable Rust releases. Bumping the
minimum supported Rust version (MSRV) is not considered a breaking change as
long as these 3 versions are still supported.
