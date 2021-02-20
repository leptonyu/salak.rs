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
    fn to_property_source(&self, path: PathBuf) -> Option<Box<dyn PropertySource>>;
    fn extention(&self) -> &'static str;
}

impl FileConfig {
    pub fn new(env: &impl Environment) -> Self {
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
        if let Some(dir) = current_dir().ok() {
            v.push(dir);
        }
        if let Some(dir) = var("HOME").ok() {
            v.push(PathBuf::from(dir));
        }
        fn _build(v: &Vec<PathBuf>, f: &str) -> Vec<PathBuf> {
            v.iter()
                .map(|d| {
                    let mut d = d.clone();
                    d.push(f.clone());
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
    ) -> Vec<Box<dyn PropertySource>> {
        self.build_path(impl_file.extention())
            .into_iter()
            .map(|path| impl_file.to_property_source(path))
            .flatten()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn build_test() {
        let fc = crate::file::FileConfig::new(&SourceRegistry::new());
        let path = fc.build_path("toml");
        assert_eq!(false, path.is_empty());
    }
}
