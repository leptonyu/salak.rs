#!/bin/bash
set -e
cargo test --verbose
cargo test --no-default-features --lib --verbose 
cargo test --no-default-features --lib --verbose --features=enable_log
cargo test --no-default-features --lib --verbose --features=enable_clap
cargo test --no-default-features --lib --verbose --features=enable_toml
cargo test --no-default-features --lib --verbose --features=enable_derive