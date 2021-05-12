use crate::*;
use std::env::current_dir;
use std::env::var;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct FileConfig {
    dir: Option<String>,
    name: String,
    profile: Option<String>,
}

pub(crate) trait FileToPropertySource {
    fn to_property_source(
        &self,
        path: PathBuf,
    ) -> Result<Option<Box<dyn PropertySource>>, PropertyError>;
    fn extention(&self) -> &'static str;
}

impl FileConfig {
    pub(crate) fn new(env: &impl Environment) -> Self {
        let _fc = Self {
            dir: env.get("app.conf.dir"),
            name: env.get_or("app.conf.name", "app".to_owned()),
            profile: env.get("app.profile"),
        };
        #[cfg(feature = "enable_log")]
        {
            if let Some(d) = &_fc.dir {
                debug!("Set APP_CONF_DIR as {}.", &d);
            }
            debug!("Set APP_CONF_NAME as {}.", &_fc.name);
        }
        _fc
    }

    fn build_path(&self, ext: &str) -> Vec<PathBuf> {
        let filename = format!("{}.{}", self.name, ext);
        let mut v = vec![];
        if let Some(dir) = &self.dir {
            v.push(PathBuf::from(dir));
        }
        if let Ok(dir) = current_dir() {
            v.push(dir);
        }
        if let Ok(dir) = var("HOME") {
            v.push(PathBuf::from(dir));
        }
        fn _build(v: &[PathBuf], f: &str) -> Vec<PathBuf> {
            v.iter()
                .map(|d| {
                    let mut d = d.clone();
                    d.push(<&str>::clone(&f));
                    d
                })
                .filter(|f| f.exists())
                .collect()
        }

        if let Some(profile_name) = self
            .profile
            .as_ref()
            .map(|p| format!("{}-{}.{}", self.name, p, ext))
        {
            let mut v1 = _build(&v, &profile_name);
            v1.append(&mut _build(&v, &filename));
            return v1;
        }
        _build(&v, &filename)
    }

    #[allow(dead_code)]
    pub(crate) fn build<T: FileToPropertySource>(
        self,
        impl_file: T,
    ) -> Result<Vec<Box<dyn PropertySource>>, PropertyError> {
        let mut v = vec![];
        for ps in self
            .build_path(impl_file.extention())
            .into_iter()
            .map(|path| impl_file.to_property_source(path))
        {
            if let Some(p) = ps? {
                v.push(p);
            }
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use crate::source::FileConfig;
    use crate::*;
    #[test]
    fn build_test() {
        let mut sr = SourceRegistry::new();
        let mut map = std::collections::BTreeMap::new();
        map.insert("app.conf.dir".to_owned(), Property::Str("src".to_owned()));
        sr.register_source(Box::new(MapPropertySource::new("xxx", map)));
        let fc = FileConfig::new(&sr);
        let path = fc.build_path("toml");
        assert_eq!(false, path.is_empty());
    }
}
