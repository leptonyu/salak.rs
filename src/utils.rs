#[doc(hidden)]
pub trait SalakStringUtil {
    fn to_prefix(&self) -> String;

    fn to_key(&self) -> String;

    fn to_first(&self) -> String;
}

impl SalakStringUtil for &str {
    fn to_prefix(&self) -> String {
        if self.is_empty() {
            self.to_owned().to_string()
        } else {
            format!("{}.", self)
        }
    }

    fn to_key(&self) -> String {
        if self.find('.').is_none()
            && !self.contains("___")
            && self.strip_prefix('_').is_none()
            && self.strip_suffix('_').is_none()
        {
            self.replace('_', ".").replace("..", "_").to_lowercase()
        } else {
            self.to_string()
        }
    }

    fn to_first(&self) -> String {
        if let Some(v) = self.find(&['.', '['][..]) {
            &self[0..v]
        } else {
            &self[..]
        }
        .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn to_prefix_test() {
        assert_eq!("", "".to_prefix());
        assert_eq!("xx.", "xx".to_prefix());
    }
    #[test]
    fn to_key_test() {
        assert_eq!("name.url", "name.url".to_key());
        assert_eq!("name.url", "NAME_URL".to_key());
        assert_eq!("name_url", "NAME__URL".to_key());
        assert_eq!("_NAME", "_NAME".to_key());
        assert_eq!("NAME_", "NAME_".to_key());
        assert_eq!("NAME__", "NAME__".to_key());
        assert_eq!("NAME___", "NAME___".to_key());
        assert_eq!("NAME___URL", "NAME___URL".to_key());
    }
    #[test]
    fn to_first_test() {
        assert_eq!("", "".to_first());
        assert_eq!("xx", "xx.".to_first());
    }
}
