use salak::*;

#[derive(FromEnvironment, Debug)]
pub struct World {
    #[field(default = "How are you?")]
    pub hey: String,
}

#[derive(FromEnvironment, Debug)]
pub struct Hello {
    pub hello: String,
    pub no: Option<String>,
    pub world: World,
}

fn main() {
    env_logger::init();

    let env = SalakBuilder::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();

    match env.require::<Hello>("") {
        Ok(h) => println!("{:?}", h),
        Err(e) => println!("{}", e),
    }
}
