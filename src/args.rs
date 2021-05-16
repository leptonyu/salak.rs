use std::collections::HashMap;

use crate::PropertyError;

/// Application info.
#[derive(Debug)]
pub struct AppInfo<'a> {
    /// Application name.
    pub name: &'a str,
    /// Application version.
    pub version: &'a str,
    /// Application authors.
    pub author: Option<&'a str>,
    /// Application description.
    pub about: Option<&'a str>,
}

/// Generate [`AppInfo`] from Cargo.toml.
#[macro_export]
macro_rules! app_info {
    () => {
        AppInfo {
            name: std::env!("CARGO_PKG_NAME"),
            version: std::env!("CARGO_PKG_VERSION"),
            author: std::option_env!("CARGO_PKG_AUTHORS"),
            about: std::option_env!("CARGO_PKG_DESCRIPTION"),
        }
    };
}

fn parse(s: String) -> Result<(String, String), PropertyError> {
    if let Some(usize) = s.find("=") {
        return Ok((s[..usize - 1].to_string(), s[usize..].to_string()));
    }
    Err(PropertyError::parse_fail("Invalid arguments"))
}

/// Generate source from args.
pub fn from_args(info: AppInfo<'_>) -> Result<HashMap<String, String>, PropertyError> {
    let mut app = clap::App::new(info.name).version(info.version).arg(
        clap::Arg::with_name("property")
            .long("property")
            .short("P")
            .value_name("KEY=VALUE")
            .help("Set properties."),
    );
    if let Some(v) = info.author {
        app = app.author(v);
    }
    if let Some(v) = info.about {
        app = app.about(v);
    }
    Ok(app
        .get_matches()
        .values_of_lossy("property")
        .unwrap_or(vec![])
        .into_iter()
        .map(|f| parse(f))
        .collect::<Result<Vec<(String, String)>, PropertyError>>()?
        .into_iter()
        .collect::<HashMap<String, String>>())
}
