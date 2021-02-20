#!/bin/bash
set -e
cargo test --verbose --all-features
cargo test --verbose --lib ## Default feature
cargo test --verbose --lib --no-default-features 
cargo test --verbose --lib --no-default-features --features=enable_log
cargo test --verbose --lib --no-default-features --features=enable_clap
cargo test --verbose --lib --no-default-features --features=enable_toml
# cargo test --verbose --lib --no-default-features --features=enable_yaml
cargo test --verbose --lib --no-default-features --features=enable_derive

cargo run --example salak --features="default enable_clap"