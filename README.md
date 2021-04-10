# salak
A layered configuration loader with zero-boilerplate configuration management.

[![Crates.io](https://img.shields.io/crates/v/salak?style=flat-square)](https://crates.io/crates/salak)
[![Crates.io](https://img.shields.io/crates/d/salak?style=flat-square)](https://crates.io/crates/salak)
[![Documentation](https://docs.rs/salak/badge.svg)](https://docs.rs/salak)
[![dependency status](https://deps.rs/repo/github/leptonyu/salak.rs/status.svg)](https://deps.rs/crate/salak)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/leptonyu/salak.rs/blob/master/LICENSE-MIT)
[![Actions Status](https://github.com/leptonyu/salak.rs/workflows/Rust/badge.svg)](https://github.com/leptonyu/salak.rs/actions)

1. [About](#about)
2. [Features](#features)
3. [Placeholder](#placeholder)
4. [Key Convension](#key-convension)
5. [Cargo Features](#cargo-features)
    1. [Default features](#default-features)
    2. [Optional features](#optional-features)
6. [Quick Example](#quick-example)


## About
`salak` is a rust version of layered configuration loader inspired by
[spring-boot](https://docs.spring.io/spring-boot/docs/current/reference/html/spring-boot-features.#boot-features-external-config).
`salak` provide an [`Environment`] structure which load properties from various [`PropertySource`]s.
Any structure which impmement [`FromEnvironment`] can get from [`Environment`] by a key.
Feature `enable_derive` provide rust attributes for auto derive [`FromEnvironment`].

## Features
Below are a few of the features which `salak` supports.

* Auto mapping properties into configuration struct.
  - `#[salak(default="value")]` set default value.
  - `#[salak(name="key")]` rename property key.
  - `#[salak(prefix="salak.database")]` set prefix.
* ** Supports load properties from various sources **
  - Support following random property key.
    - `random.u8`
    - `random.u16`
    - `random.u32`
    - `random.i8`
    - `random.i16`
    - `random.i32`
    - `random.i64`
  - Load properties from command line arguments.
  - Load properties from system environment.
  - Load properties from toml config file.
  - Load properties from yaml config file.
  - Easy to add a new property source.
* Supports profile(develop/production) based configuration.
* Supports placeholder resolve.
* Supports reload configurations.

## Placeholder

* `${key:default}` means get value of `key`, if not exists then return `default`.
* `${key}` means get value of `key`, if not exists then return `PropertyError::NotFound(_)`.
* `\$\{key\}` means escape to `${key}`.

## Key Convension
* `a.b.c` is a normal key separated by dot(`.`).
* `a.b[0]`, `a.b[1]`, `a.b[2]`... is a group of keys with arrays.

## Cargo Features

### Default features
1. `enable_log`, enable log record if enabled.
2. `enable_toml`, enable toml support.
3. `enable_derive`, enable auto derive [`FromEnvironment`] for struts.

### Optional features
1. `enable_pico`, enable default command line arguments parsing by `pico-args`.
2. `enable_clap`, enable default command line arguments parsing by `clap`.
3. `enable_yaml`, enable yaml support.
4. `enable_rand`, enable random value support.

## Quick Example

```rust
use salak::*;

#[derive(FromEnvironment, Debug)]
pub struct SslConfig {
    key: String,
    pem: String,
}

#[derive(FromEnvironment, Debug)]
#[salak(prefix = "database")]
pub struct DatabaseConfig {
  url: String,
  #[salak(default = "salak")]
  username: String,
  password: Option<String>,
  description: String,
  #[salak(name="ssl")]
  ssl_config: Option<SslConfig>,  
}

std::env::set_var("database.url", "localhost:5432");
std::env::set_var("database.description", "\\$\\{Hello\\}");
let env = Salak::new()
   .with_default_args(auto_read_sys_args_param!()) // This line need enable feature `enable_clap`.
   .build();

match env.load_config::<DatabaseConfig>() {
    Ok(val) => println!("{:?}", val),
    Err(e) => println!("{}", e),
}

// Output: DatabaseConfig {
//  url: "localhost:5432",
//  username: "salak",
//  password: None,
//  description: "${Hello}",
//  ssl_config: None,
// }
```