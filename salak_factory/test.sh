#!/bin/bash

cd $(dirname $0)

cargo test --verbose --all-features
cargo test --verbose --lib ## Default feature
cargo test --verbose --lib --no-default-features --features=redis_default
cargo test --verbose --lib --no-default-features --features=redis_cluster
cargo test --verbose --lib --no-default-features --features=postgresql
cargo test --verbose --lib --no-default-features --features=metric

cargo run --example redis --features='default redis_default salak/all log' -- -h