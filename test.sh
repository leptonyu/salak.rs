#!/bin/bash
set -e
cargo test --verbose
cargo test --verbose --all-features
cargo test --verbose --lib ## Default feature
cargo test --verbose --lib --no-default-features 
cargo test --verbose --lib --no-default-features --features=toml
cargo test --verbose --lib --no-default-features --features=yaml
cargo test --verbose --lib --no-default-features --features=derive

# cargo run --example salak

# cd salak_factory
# cargo test --verbose --all-features
# cargo test --verbose --lib ## Default feature
# cargo test --verbose --lib --no-default-features --features=enable_redis
# cargo test --verbose --lib --no-default-features --features=enable_redis_cluster
# cargo test --verbose --lib --no-default-features --features=enable_postgres
# cargo test --verbose --lib --no-default-features --features=enable_log

# cargo run --example redis --features='default enable_log enable_redis'
# cd -
