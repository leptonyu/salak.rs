//! Provide command line arguments [`PropertySource`].
use crate::*;
#[cfg(any(feature = "enable_clap", feature = "enable_pico"))]
use regex::Regex;
use std::collections::BTreeMap;

/// Command line arguments parser mode.
#[derive(Debug)]
pub enum SysArgsMode {
    #[cfg(any(feature = "enable_clap", feature = "enable_pico"))]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "enable_clap", feature = "enable_pico")))
    )]
    /// Use default `pico-args` parser. It has a OPTION named `-P` to set customized properties.
    ///
    /// ```no_run
    /// use salak::*;
    /// let env = Salak::new()
    ///    .with_default_args(auto_read_sys_args_param!())
    ///    .build();
    ///
    /// // Command line output:
    /// // salak 0.0.0
    /// // Daniel Yu <leptonyu@gmail.com>
    /// // A rust configuration loader
    /// //
    /// // USAGE:
    /// //     salak [OPTIONS]
    /// //
    /// // FLAGS:
    /// //     -h, --help       Prints help information
    /// //     -V, --version    Prints version information
    /// //
    /// // OPTIONS:
    /// //     -P, --property <KEY=VALUE>...    Set properties
    /// ```
    Auto(SysArgsParam),
    /// Customize command line arguments parser, and provide a `Vec` to [`PropertySource`].
    /// If you can use any cli parser.
    ///
    /// ```no_run
    /// use salak::*;
    /// let arg_props = vec![];  // replace `vec![]` with your cli parser process result.
    /// let env = Salak::new()
    ///    .with_custom_args(arg_props)
    ///    .build();
    /// ```
    Custom(Vec<(String, Property)>),
}

/// Command line help info, such as name, version, author, etc.
#[cfg(any(feature = "enable_clap", feature = "enable_pico"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "enable_clap", feature = "enable_pico")))
)]
#[derive(Debug, Copy, Clone)]
pub struct SysArgsParam {
    /// App name.
    pub name: &'static str,
    /// App version.
    pub version: &'static str,
    /// App authors.
    pub author: Option<&'static str>,
    /// App description.
    pub about: Option<&'static str>,
}

/// Auto generate [`SysArgsParam`] from Cargo.toml.
///
/// Due to macro [`env!`] will generate value at compile time, so users should call it at final project.
#[macro_export]
#[cfg(any(feature = "enable_clap", feature = "enable_pico"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "enable_clap", feature = "enable_pico")))
)]
macro_rules! auto_read_sys_args_param {
    () => {
        SysArgsParam {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            author: option_env!("CARGO_PKG_AUTHORS"),
            about: option_env!("CARGO_PKG_DESCRIPTION"),
        }
    };
}

/// A simple implementation of [`PropertySource`].
pub(crate) struct SysArgs(pub(crate) MapPropertySource);

impl SysArgs {
    /// Create [`SysArgs`].
    #[allow(clippy::infallible_destructuring_match)]
    pub(crate) fn new(args: SysArgsMode) -> Self {
        let mut map = BTreeMap::new();
        if let SysArgsMode::Auto(arg) = &args {
            map.insert("app.name".to_string(), arg.name.into_property());
            map.insert("app.version".to_string(), arg.version.into_property());
        }
        let args = match args {
            #[cfg(feature = "enable_clap")]
            #[cfg(not(feature = "enable_pico"))]
            SysArgsMode::Auto(arg) => Self::parse_args_by_clap(arg),
            #[cfg(feature = "enable_pico")]
            SysArgsMode::Auto(arg) => Self::parse_args_by_pico(arg),
            SysArgsMode::Custom(arg) => arg,
        };

        for (k, v) in args {
            map.insert(k, v);
        }
        SysArgs(MapPropertySource::new("SystemArguments", map))
    }

    #[cfg(feature = "enable_clap")]
    #[cfg(not(feature = "enable_pico"))]
    fn parse_args_by_clap(param: SysArgsParam) -> Vec<(String, Property)> {
        let mut app = clap::App::new(param.name).version(param.version);
        if let Some(a) = param.author {
            app = app.author(a);
        }
        if let Some(a) = param.about {
            app = app.about(a);
        }
        let matches = app
            .arg(
                clap::Arg::with_name("property")
                    .short("P")
                    .long("property")
                    .value_name("KEY=VALUE")
                    .multiple(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .help("Set properties"),
            )
            .get_matches();
        Self::parse_properties(matches.values_of_lossy("property").unwrap_or_default())
    }

    #[cfg(feature = "enable_pico")]
    fn parse_args_by_pico(param: SysArgsParam) -> Vec<(String, Property)> {
        let mut title = "".to_owned();
        if let Some(author) = param.author {
            title.push('\n');
            title.push_str(author);
        }
        if let Some(about) = param.about {
            title.push('\n');
            title.push_str(about);
        }
        let help = format!(
            "\
{} {}{}
USAGE:
  {} [OPTIONS]

FLAGS:
  -h, --help            Prints help information

OPTIONS:
  -P, --property <KEY=VALUE>...    Set properties
",
            param.name, param.version, title, param.name
        );
        let mut pargs = pico_args::Arguments::from_env();

        // Help has a higher priority and should be handled separately.
        if pargs.contains(["-h", "--help"]) {
            print!("{}", &help);
            std::process::exit(0);
        }

        Self::parse_properties(
            pargs
                .values_from_str(["-P", "--property"])
                .unwrap_or_default(),
        )
    }

    #[cfg(any(feature = "enable_clap", feature = "enable_pico"))]
    fn parse_properties(iter: Vec<String>) -> Vec<(String, Property)> {
        lazy_static::lazy_static! {
            static ref RE: Regex = Regex::new(
                r"^([^=]+)=(.+)$"
            )
            .expect(NOT_POSSIBLE);
        }
        iter.iter()
            .flat_map(|k: &String| match RE.captures(k) {
                Some(ref v) => Some((
                    v.get(1).expect(NOT_POSSIBLE).as_str().to_owned(),
                    Property::Str(v.get(2).expect(NOT_POSSIBLE).as_str().to_owned()),
                )),
                _ => None,
            })
            .collect()
    }
}

#[cfg(feature = "enable_clap")]
#[cfg(not(feature = "enable_pico"))]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_clap")))]
impl Default for SysArgs {
    /// A simple implementation using `clap`.
    fn default() -> Self {
        SysArgs::new(SysArgsMode::Auto(auto_read_sys_args_param!()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "enable_clap")]
    fn test_auto_read_sys_args_param() {
        use crate::*;
        let m = auto_read_sys_args_param!();
        assert_eq!("salak", m.name);
    }
}
