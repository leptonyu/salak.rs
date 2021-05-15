# salak
Salak is a multi layered configuration loader and zero-boilerplate configuration parser, with many predefined sources.

[![Crates.io](https://img.shields.io/crates/v/salak?style=flat-square)](https://crates.io/crates/salak)
[![Crates.io](https://img.shields.io/crates/d/salak?style=flat-square)](https://crates.io/crates/salak)
[![Documentation](https://docs.rs/salak/badge.svg)](https://docs.rs/salak)
[![dependency status](https://deps.rs/repo/github/leptonyu/salak.rs/status.svg)](https://deps.rs/crate/salak)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/leptonyu/salak.rs/blob/master/LICENSE-MIT)
[![Actions Status](https://github.com/leptonyu/salak.rs/workflows/Rust/badge.svg)](https://github.com/leptonyu/salak.rs/actions)

Please refer to [salak doc](https://docs.rs/salak).

## *Notice*
Please *notice* that `salak-0.9.0` is totally rewrited, so the APIs may changes much, and some functions may be removed. Some will be added in later version.

## Quick Example
```rust
use salak::*;

#[derive(Debug, FromEnvironment)]
#[salak(prefix = "config")]
struct Config {
    #[salak(default = false)]
    verbose: bool,
    optional: Option<String>,
    #[salak(name = "val")]
    value: i64,
}
let env = Salak::builder()
    .set("config.val", "2021")
    .unwrap_build();
let config = env.get::<Config>().unwrap();
assert_eq!(2021, config.value);
assert_eq!(None, config.optional);
assert_eq!(false, config.verbose);
```


## Salak Factory
[salak_factory](https://crates.io/crates/salak_factory) is out-of-box crate for using well known components, such as redis, postgresql, etc.
```rust
use salak::*;
use salak_factory::{redis_default::RedisConfig, Factory};

fn main() -> Result<(), PropertyError> {
    let env = Salak::new()?;
    let redis_pool = env.build::<RedisConfig>()?;
    let redis_conn = redis_pool.get().unwrap();
    let _: u64 = redis_conn.set("hello", 1u64).unwrap();
    Ok(())
}
```