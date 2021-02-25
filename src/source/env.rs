//! Provide system environment [`PropertySource`].
use crate::utils::SalakStringUtil;
use crate::*;
use std::collections::BTreeMap;

/// [`PropertySource`] read properties from system environment.
#[derive(Debug, Clone)]
pub struct SysEnvPropertySource(MapPropertySource);

impl SysEnvPropertySource {
    pub(crate) fn new() -> Self {
        let mut map = BTreeMap::new();
        for (k, v) in std::env::vars() {
            let k: &str = &k;
            let k2 = k.to_key();
            if k2 != k {
                map.insert(k.to_owned(), Property::Str(v.clone()));
            }
            map.insert(k2, Property::Str(v));
        }
        Self(MapPropertySource::new("SystemEnvironment", map))
    }
}

impl PropertySource for SysEnvPropertySource {
    fn name(&self) -> String {
        self.0.name()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        self.0.get_property(name)
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn find_keys(&self, prefix: &str) -> Vec<String> {
        self.0.find_keys(prefix)
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::*;
//     #[test]
//     fn name_tests() {
//         let v: HashSet<String> = SysEnvPropertySource::normalize_keys("name.url")
//             .into_iter()
//             .collect();
//         assert_eq!(true, v.contains("name.url"));
//         assert_eq!(true, v.contains("NAME_URL"));
//         let v: HashSet<String> = SysEnvPropertySource::normalize_keys("NAME_URL")
//             .into_iter()
//             .collect();
//         assert_eq!(true, v.contains("name.url"));
//         assert_eq!(true, v.contains("NAME_URL"));

//         let v: HashSet<String> = SysEnvPropertySource::normalize_keys("name[1].url")
//             .into_iter()
//             .collect();
//         assert_eq!(true, v.contains("name[1].url"));
//         assert_eq!(true, v.contains("NAME_1_URL"));
//         let v: HashSet<String> = SysEnvPropertySource::normalize_keys("NAME_1_URL")
//             .into_iter()
//             .collect();
//         assert_eq!(true, v.contains("name.1.url"));
//         assert_eq!(true, v.contains("NAME_1_URL"));

//         let v: HashSet<String> = SysEnvPropertySource::normalize_keys("name[1][2].url")
//             .into_iter()
//             .collect();
//         assert_eq!(true, v.contains("name[1][2].url"));
//         assert_eq!(true, v.contains("NAME_1_2_URL"));
//         let v: HashSet<String> = SysEnvPropertySource::normalize_keys("NAME_1_2_URL")
//             .into_iter()
//             .collect();
//         assert_eq!(true, v.contains("name.1.2.url"));
//         assert_eq!(true, v.contains("NAME_1_2_URL"));

//         let v: HashSet<String> = SysEnvPropertySource::normalize_keys("name_family.url")
//             .into_iter()
//             .collect();
//         assert_eq!(true, v.contains("name_family.url"));
//         assert_eq!(true, v.contains("NAME__FAMILY_URL"));
//         let v: HashSet<String> = SysEnvPropertySource::normalize_keys("NAME__FAMILY_URL")
//             .into_iter()
//             .collect();
//         assert_eq!(true, v.contains("name_family.url"));
//         assert_eq!(true, v.contains("NAME__FAMILY_URL"));
//     }
// }
