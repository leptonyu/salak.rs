use salak::*;

#[derive(FromEnvironment, Debug)]
#[field(prefix = "a.b.c")]
pub struct Hello {
    #[field(default = "world")]
    pub hello: String,
    #[field()]
    pub no: Option<String>,
    #[field()]
    pub hey: Option<i64>,
}

fn main() {
    env_logger::init();

    let env = SalakBuilder::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();

    match env.load::<Hello>() {
        Ok(h) => println!("{:?}", h),
        Err(e) => println!("{}", e),
    }
}
