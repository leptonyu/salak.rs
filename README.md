# `salak`, A rust configuration loader.

[![Crates.io](https://img.shields.io/crates/v/salak?style=flat-square)](https://crates.io/crates/salak)
[![Crates.io](https://img.shields.io/crates/d/salak?style=flat-square)](https://crates.io/crates/salak)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/leptonyu/salak.rs/blob/master/LICENSE-MIT)
[![Actions Status](https://github.com/leptonyu/salak.rs/workflows/Rust/badge.svg)](https://github.com/leptonyu/salak.rs/actions)

`salak` try to provide an out-of-box configuration loader for creating new apps, such as cli, web, servers.

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