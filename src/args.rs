use std::collections::HashMap;

use crate::{derive::KeyDescs, KeyDesc, PropertyError, Res};

/// Application info.
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "args")))]
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
#[cfg_attr(docsrs, doc(cfg(feature = "args")))]
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

fn parse(s: String) -> Res<(String, String)> {
    if let Some(usize) = s.find("=") {
        return Ok((s[..usize - 1].to_string(), s[usize..].to_string()));
    }
    Err(PropertyError::parse_fail("Invalid arguments"))
}

#[cfg_attr(docsrs, doc(cfg(feature = "args")))]
/// Generate source from args.
pub(crate) fn from_args(desc: Vec<KeyDesc>, info: AppInfo<'_>) -> Res<HashMap<String, String>> {
    let help = format!("KEYS:\n{}\n", &KeyDescs(desc));

    let mut app = clap::App::new(info.name)
        .version(info.version)
        .arg(
            clap::Arg::with_name("property")
                .long("property")
                .short("P")
                .value_name("KEY=VALUE")
                .multiple(true)
                .help("Set properties."),
        )
        .after_help(help.as_str());
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
        .collect::<Res<Vec<(String, String)>>>()?
        .into_iter()
        .collect::<HashMap<String, String>>())
}
