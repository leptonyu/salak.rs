# salak
A layered configuration loader with zero-boilerplate configuration management.

[![Crates.io](https://img.shields.io/crates/v/salak?style=flat-square)](https://crates.io/crates/salak)
[![Crates.io](https://img.shields.io/crates/d/salak?style=flat-square)](https://crates.io/crates/salak)
[![Documentation](https://docs.rs/salak/badge.svg)](https://docs.rs/salak)
[![dependency status](https://deps.rs/repo/github/leptonyu/salak.rs/status.svg)](https://deps.rs/crate/salak)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/leptonyu/salak.rs/blob/master/LICENSE-MIT)
[![Actions Status](https://github.com/leptonyu/salak.rs/workflows/Rust/badge.svg)](https://github.com/leptonyu/salak.rs/actions)

### About
`salak` try to provide an out-of-box configuration loader for creating new apps, such as cli, web, servers.

### Multi-Layered source environment.
`salak` defines following default `PropertySource`s:
1. Command line arguments using `clap` to parsing `-P, --propery KEY=VALUE`.
2. System Environment.
3. app.toml(*) in current dir and $HOME dir. Or if you specify `APP_CONF_DIR` dir, then only load toml in this dir.

\* `APP_CONF_NAME` can be specified to replace `app`.

### Placeholder format
1. `${key:default}` means get value of `key`, if not exists then return `default`.
2. `${key}` means get value of `key`, if not exists then return `PropertyError::NotFound(_)`.

### Key format
1. `a.b.c` is a normal key separated by dot(`.`).
2. `a.b[0]`, `a.b[1]`, `a.b[2]`... is a group of keys with arrays.
3. System environment key will be changed from `HELLO_WORLD` <=> `hello.world`, `HELLO__WORLD_HOW` <=> `hello_world.how`, `hello[1].world` => `HELLO_1_WORLD` <=> `hello.1.world`.

### Auto derived parameters.

##### attribute `default` to set default value.
1. `#[salak(default="string")]`
2. `#[salak(default=1)]`

### Features

##### Default features
1. `enable_log`, enable log record if enabled.
2. `enable_toml`, enable toml support.
3. `enable_derive`, enable auto derive [`FromEnvironment`] for struts.

##### Optional features
1. `enable_clap`, enable default command line arguments parsing by `clap`.
2. `enable_yaml`, enable yaml support.


### Quick Code
```rust
use salak::*;
#[derive(FromEnvironment, Debug)]
pub struct DatabaseConfig {
    url: String,
    #[salak(default = "salak")]
    username: String,
    password: Option<String>,
    description: String,
}

fn main() {
  std::env::set_var("database.url", "localhost:5432");
  std::env::set_var("database.description", "\\$\\{Hello\\}");
  let env = Salak::new()
     .with_default_args(auto_read_sys_args_param!()) // This line need enable feature `enable_clap`.
     .build();
 
  match env.require::<DatabaseConfig>("database") {
      Ok(val) => println!("{:?}", val),
      Err(e) => println!("{}", e),
  }
}
// Output: DatabaseConfig { url: "localhost:5432", username: "salak", password: None, description: "${Hello}" }
```

### Quick Run
```bash
git clone https://github.com/leptonyu/salak.rs.git
cd salak.rs
cargo run --example salak --features="default enable_clap" -- -h
# salak 0.5.0
# Daniel Yu <leptonyu@gmail.com>
# A rust configuration loader
# 
# USAGE:
#     salak [OPTIONS]
# 
# FLAGS:
#     -h, --help       Prints help information
#     -V, --version    Prints version information
# 
# OPTIONS:
#     -P, --property <KEY=VALUE>...    Set properties
```

### TODO
1. Reload configurations.