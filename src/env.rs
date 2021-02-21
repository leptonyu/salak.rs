//! Provide system environment [`PropertySource`].
use crate::*;

/// A wrapper of [`PropertySource`] for getting properties from system environment.
#[derive(Debug, Copy, Clone)]
pub struct SysEnv;

impl SysEnv {
    fn normalize_keys(name: &str) -> Vec<String> {
        let mut v = vec![name.to_owned()];
        if name.find('.').is_some() {
            let name = name
                .replace('_', "__")
                .replace("]", "")
                .replace(&['.', '['][..], "_")
                .to_uppercase();
            v.push(name);
        } else {
            let name = name.replace('_', ".").replace("..", "_").to_lowercase();
            v.push(name);
        }
        v
    }
}

impl PropertySource for SysEnv {
    fn name(&self) -> String {
        "SystemEnvironment".to_owned()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        for n in Self::normalize_keys(name) {
            if let Ok(v) = std::env::var(n) {
                return Some(Property::Str(v));
            }
        }
        None
    }
    fn is_empty(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::env::*;
    #[test]
    fn name_tests() {
        let v: HashSet<String> = SysEnv::normalize_keys("name.url").into_iter().collect();
        assert_eq!(true, v.contains("name.url"));
        assert_eq!(true, v.contains("NAME_URL"));
        let v: HashSet<String> = SysEnv::normalize_keys("NAME_URL").into_iter().collect();
        assert_eq!(true, v.contains("name.url"));
        assert_eq!(true, v.contains("NAME_URL"));

        let v: HashSet<String> = SysEnv::normalize_keys("name[1].url").into_iter().collect();
        assert_eq!(true, v.contains("name[1].url"));
        assert_eq!(true, v.contains("NAME_1_URL"));
        let v: HashSet<String> = SysEnv::normalize_keys("NAME_1_URL").into_iter().collect();
        assert_eq!(true, v.contains("name.1.url"));
        assert_eq!(true, v.contains("NAME_1_URL"));

        let v: HashSet<String> = SysEnv::normalize_keys("name[1][2].url")
            .into_iter()
            .collect();
        assert_eq!(true, v.contains("name[1][2].url"));
        assert_eq!(true, v.contains("NAME_1_2_URL"));
        let v: HashSet<String> = SysEnv::normalize_keys("NAME_1_2_URL").into_iter().collect();
        assert_eq!(true, v.contains("name.1.2.url"));
        assert_eq!(true, v.contains("NAME_1_2_URL"));

        let v: HashSet<String> = SysEnv::normalize_keys("name_family.url")
            .into_iter()
            .collect();
        assert_eq!(true, v.contains("name_family.url"));
        assert_eq!(true, v.contains("NAME__FAMILY_URL"));
        let v: HashSet<String> = SysEnv::normalize_keys("NAME__FAMILY_URL")
            .into_iter()
            .collect();
        assert_eq!(true, v.contains("name_family.url"));
        assert_eq!(true, v.contains("NAME__FAMILY_URL"));
    }
}
