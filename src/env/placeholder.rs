use crate::*;

/// An implementation of [`Environment`] that can resolve placeholder for values.
///
/// ```
/// use salak::*;
/// std::env::set_var("v1", "value");
/// std::env::set_var("v2", "${v1}");
/// std::env::set_var("v3", "${no_found:default}");
/// std::env::set_var("v4", "${no_found:${v2}}");
/// let env = PlaceholderResolver::new(true, SourceRegistry::default());
/// assert_eq!("value", &env.require::<String>("v1").unwrap());
/// assert_eq!("value", &env.require::<String>("v2").unwrap());
/// assert_eq!("default", &env.require::<String>("v3").unwrap());
/// assert_eq!("value", &env.require::<String>("v4").unwrap());
/// ```
#[derive(Debug)]
pub struct PlaceholderResolver<T: Environment> {
    enabled: bool,
    pub(crate) env: T,
    placeholder_prefix: char,
    placeholder_suffix: char,
    placeholder_middle: char,
}

impl<E: Environment> PlaceholderResolver<E> {
    /// Create placeholder environment.
    pub fn new(enabled: bool, env: E) -> Self {
        PlaceholderResolver {
            enabled,
            env,
            placeholder_prefix: '{',
            placeholder_suffix: '}',
            placeholder_middle: ':',
        }
    }

    fn require_with_parse<T: FromEnvironment>(
        &self,
        name: &str,
        contains: &mut HashSet<String>,
    ) -> Result<T, PropertyError> {
        if !contains.insert(name.to_owned()) {
            return Err(PropertyError::RecursiveParse(name.to_owned()));
        }
        let p = match self.env.require::<Option<Property>>(name)? {
            Some(Property::Str(s)) => self.parse_value(&s, contains)?,
            v => v,
        };
        T::from_env(name, p, self)
    }

    fn parse_value(
        &self,
        mut val: &str,
        contains: &mut HashSet<String>,
    ) -> Result<Option<Property>, PropertyError> {
        let mut stack: Vec<String> = vec![];
        let mut pre = "".to_owned();
        let placeholder: &[_] = &['$', '\\', self.placeholder_suffix];
        let prefix = &self.placeholder_prefix.to_string();
        while let Some(left) = val.find(placeholder) {
            match &val[left..=left] {
                "$" => {
                    let (push, next) =
                        if val.len() == left + 1 || &val[left + 1..=left + 1] != prefix {
                            (&val[..=left], &val[left + 1..])
                        } else {
                            (&val[..left], &val[left + 2..])
                        };
                    if stack.is_empty() {
                        pre.push_str(push);
                        stack.push("".to_owned());
                    } else {
                        stack.push(push.to_string());
                    }
                    val = next;
                }
                "\\" => {
                    if val.len() == left + 1 {
                        return Err(PropertyError::parse_failed("End with single \\"));
                    }
                    let merge = format!("{}{}", &val[..left], &val[left + 1..=left + 1]);
                    if let Some(mut v) = stack.pop() {
                        v.push_str(&merge);
                        stack.push(v);
                    } else {
                        pre.push_str(&merge);
                    }
                    val = &val[left + 2..];
                }
                _ => {
                    if let Some(mut name) = stack.pop() {
                        name.push_str(&val[..left]);
                        let mut def: Option<String> = None;
                        let key = if let Some(k) = name.find(self.placeholder_middle) {
                            def = Some(name[k + 1..].to_owned());
                            &name[..k]
                        } else {
                            &name
                        };
                        let value = if let Some(d) = def {
                            self.require_with_parse::<Option<String>>(&key, contains)?
                                .unwrap_or(d)
                        } else {
                            self.require_with_parse::<String>(&key, contains)?
                        };
                        if let Some(mut prefix) = stack.pop() {
                            prefix.push_str(&value);
                            stack.push(prefix);
                        } else {
                            pre.push_str(&value);
                        }
                    } else {
                        return Err(PropertyError::parse_failed("Suffix not match 1"));
                    }
                    val = &val[left + 1..];
                }
            }
        }
        if !stack.is_empty() {
            return Err(PropertyError::parse_failed("Suffix not match 2"));
        }
        pre.push_str(&val);
        Ok(Some(Property::Str(pre)))
    }
}

impl<E: Environment> Environment for PlaceholderResolver<E> {
    fn contains(&self, name: &str) -> bool {
        self.env.contains(name)
    }

    fn require<T>(&self, name: &str) -> Result<T, PropertyError>
    where
        T: FromEnvironment,
    {
        if self.enabled && !name.is_empty() {
            self.require_with_parse::<T>(name, &mut HashSet::new())
        } else {
            self.env.require(name)
        }
    }

    fn resolve_placeholder(&self, value: String) -> Result<Option<Property>, PropertyError> {
        self.parse_value(&value, &mut HashSet::new())
    }
    fn find_keys(&self, prefix: &str) -> Vec<String> {
        self.env.find_keys(prefix)
    }
}

#[cfg(test)]
mod tests {

    use crate::*;

    #[test]
    fn check() {
        std::env::set_var("v1", "value");
        std::env::set_var("v2", "${v1}");
        std::env::set_var("v3", "${no_found:default}");
        std::env::set_var("v4", "${no_found:${v2}}");
        std::env::set_var("v5", "${no_found:${no_found_2:hello}}");
        std::env::set_var("v6", "hello-${v1}-${v3}-");
        std::env::set_var("v7", "${v7}");
        std::env::set_var("v10", "${no_found}");
        std::env::set_var("v11", "\\{raw\\}");
        let env = PlaceholderResolver::new(true, SourceRegistry::default());
        assert_eq!("value", &env.require::<String>("v1").unwrap());
        assert_eq!("value", &env.require::<String>("v2").unwrap());
        assert_eq!("default", &env.require::<String>("v3").unwrap());
        assert_eq!("value", &env.require::<String>("v4").unwrap());
        assert_eq!("hello", &env.require::<String>("v5").unwrap());
        assert_eq!(
            "hello-value-default-",
            &env.require::<String>("v6").unwrap()
        );

        let v7 = env.require::<String>("v7");

        assert_eq!(true, v7.is_err());
        assert_eq!(
            PropertyError::RecursiveParse("v7".to_string()),
            v7.unwrap_err()
        );

        let v8 = env.require::<Option<String>>("v8");
        assert_eq!(true, v8.is_ok());
        let v9 = env.require::<Option<String>>("");
        assert_eq!(true, v9.is_ok());
        assert_eq!(None, v9.unwrap());

        let v10 = env.require::<String>("v10");
        assert_eq!(true, v10.is_err());
        assert_eq!(
            PropertyError::NotFound("no_found".to_owned()),
            v10.unwrap_err()
        );
        assert_eq!(Ok("{raw}".to_owned()), env.require::<String>("v11"));
    }
}
