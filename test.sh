#!/bin/bash
set -e
cargo test --verbose
cargo test --verbose --all-features
cargo test --verbose --lib ## Default feature
cargo test --verbose --lib --no-default-features 
cargo test --verbose --lib --no-default-features --features=enable_log
cargo test --verbose --lib --no-default-features --features=enable_clap
cargo test --verbose --lib --no-default-features --features=enable_pico
cargo test --verbose --lib --no-default-features --features=enable_toml
cargo test --verbose --lib --no-default-features --features=enable_yaml
cargo test --verbose --lib --no-default-features --features=enable_derive

cargo run --example salak

cd salak_factory
cargo test --verbose
cargo test --verbose --all-features
cargo test --verbose --lib ## Default feature
cargo test --verbose --lib --no-default-features --features=enable_redis
cargo test --verbose --lib --no-default-features --features=enable_redis_cluster
cargo test --verbose --lib --no-default-features --features=enable_postgres

cargo run --example redis --features='default enable_redis'
cd -