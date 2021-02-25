#[doc(hidden)]
pub trait SalakStringUtil {
    fn to_prefix(&self) -> String;

    fn to_key(&self) -> String;
    fn to_env_var(&self) -> String;

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
            || self.strip_prefix('_').is_none()
            || self.strip_suffix('_').is_none()
        {
            self.replace('_', ".").replace("..", "_").to_lowercase()
        } else {
            self.to_string()
        }
    }

    fn to_env_var(&self) -> String {
        self.replace('_', "__")
            .replace("]", "")
            .replace(&['.', '['][..], "_")
            .to_uppercase()
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
