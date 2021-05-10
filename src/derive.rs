use crate::*;

#[doc(hidden)]
pub trait AutoDeriveFromEnvironment: FromEnvironment {}

impl<P: AutoDeriveFromEnvironment> AutoDeriveFromEnvironment for Option<P> {}

#[doc(hidden)]
pub trait DefaultSourceFromEnvironment: AutoDeriveFromEnvironment {
    fn prefix() -> &'static str;
}

impl<P: DefaultSourceFromEnvironment> DefaultSourceFromEnvironment for Option<P> {
    fn prefix() -> &'static str {
        P::prefix()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[derive(Debug, FromEnvironment)]
    struct SubEmpty {}
    #[derive(Debug, FromEnvironment)]
    struct SubOption {
        field: u8,
    }

    #[derive(Debug, FromEnvironment)]
    #[salak(prefix = "")]
    struct Config {
        f_option: Option<SubOption>,
        #[salak(default = 0)]
        f_u8: u8,
        #[salak(default = 0)]
        f_u16: u16,
        #[salak(default = 0)]
        f_u32: u32,
        #[salak(default = 0)]
        f_u64: u64,
        #[salak(default = 0)]
        f_u128: u128,
        #[salak(default = 0)]
        f_usize: usize,
        #[salak(default = 0)]
        f_i8: i8,
        #[salak(default = 0)]
        f_i16: i16,
        #[salak(default = 0)]
        f_i32: i32,
        #[salak(default = 0)]
        f_i64: i64,
        #[salak(default = 0)]
        f_isize: isize,
        #[salak(default = 0)]
        f_f32: f32,
        #[salak(default = 0)]
        f_f64: f64,
        #[salak(default = 0)]
        f_str: String,
        #[salak(default = 0)]
        f_bool: bool,
        #[salak(default = 0)]
        f_property: Property,
        #[salak(default = "0s")]
        f_duration: std::time::Duration,
        f_option_u8: Option<u8>,
        f_option_u16: Option<u16>,
        f_option_u32: Option<u32>,
        f_option_u64: Option<u64>,
        f_option_u128: Option<u128>,
        f_option_usize: Option<usize>,
        f_option_i8: Option<i8>,
        f_option_i16: Option<i16>,
        f_option_i32: Option<i32>,
        f_option_i64: Option<i64>,
        f_option_isize: Option<isize>,
        f_option_f32: Option<f32>,
        f_option_f64: Option<f64>,

        f_option_empty: Option<SubEmpty>,
        f_sub: SubOption,
    }

    #[test]
    fn compile_test() {
        let env = Salak::new().build();
        let config = env.load_config::<Config>();
        println!("{:?}", config);
        println!("{:?}", env.require::<Option<SubOption>>(""));
        assert_eq!(true, config.is_ok());
    }
}
