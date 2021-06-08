#!/bin/bash
set -e
cargo test --verbose
cargo test --verbose --all-features
cargo test --verbose --lib ## Default feature
cargo test --verbose --lib --no-default-features 
cargo test --verbose --lib --no-default-features --features=toml
cargo test --verbose --lib --no-default-features --features=yaml
cargo test --verbose --lib --no-default-features --features=derive
cargo test --verbose --lib --no-default-features --features=args
cargo test --verbose --lib --no-default-features --features=app

cargo bench --all-features 
cargo run --example salak --features='default args app log' -- -h
