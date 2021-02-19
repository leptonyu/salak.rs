# salak
A configuration loader with zero-boilerplate configuration management.

[![Crates.io](https://img.shields.io/crates/v/salak?style=flat-square)](https://crates.io/crates/salak)
[![Crates.io](https://img.shields.io/crates/d/salak?style=flat-square)](https://crates.io/crates/salak)
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
`salak` use format `{key:default}` to reference to other `key`, and if `key` not exists then use value `default`.

### Key format
1. `a.b.c` is a normal key separated by dot(`.`).
2. `a.b.0`, `a.b.1`, `a.b.2`... is a group of keys with arrays.
3. System environment key will be changed from `HELLO_WORLD` to `hello.world`, vice versa.

### Auto derived parameters.

##### attribute `default` to set default value.
1. `#[salak(default="string")]`
2. `#[salak(default=1)]`

##### attribute `disable_placeholder` to disable placeholder parsing.
1. `#[salak(disable_placeholder)]`
2. `#[salak(disable_placeholder = true)]`

### Quick Code
```rust
use salak::*;
#[derive(FromEnvironment, Debug)]
pub struct DatabaseConfig {
    url: String,
    #[salak(default = "salak")]
    name: String,
    #[salak(default = "{database.name}")]
    username: String,
    password: Option<String>,
    #[salak(default = "{Hello}", disable_placeholder)]
    description: String,
}

fn main() {
  std::env::set_var("database.url", "localhost:5432");
  let env = SalakBuilder::new()
     .with_default_args(auto_read_sys_args_param!())
     .build();
 
  match env.require::<DatabaseConfig>("database") {
      Ok(val) => println!("{:?}", val),
      Err(e) => println!("{}", e),
  }
}
// Output: DatabaseConfig { url: "localhost:5432", name: "salak", username: "salak", password: None, description: "{Hello}" }
```

### Quick Run
```bash
git clone https://github.com/leptonyu/salak.rs.git
cd salak.rs
cargo run --example salak -- -h
# salak 0.2.0
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
1. Support hashmap.
2. Support toml date.
3. Reload configurations.