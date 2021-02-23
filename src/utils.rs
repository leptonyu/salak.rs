#[doc(hidden)]
pub trait SalakStringUtil {
    fn to_prefix(&self) -> String;
}

impl SalakStringUtil for &str {
    fn to_prefix(&self) -> String {
        if self.is_empty() {
            self.to_owned().to_string()
        } else {
            format!("{}.", self)
        }
    }
}
