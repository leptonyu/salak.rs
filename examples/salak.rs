use salak::*;

#[derive(FromEnvironment, Debug)]
pub struct DatabaseConfig {
    url: String,
    #[field(default = "salak")]
    username: String,
    password: Option<String>,
}

fn main() {
    env_logger::init();
    std::env::set_var("database.url", "localhost:5432");
    let env = SalakBuilder::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();

    match env.require::<DatabaseConfig>("database") {
        Ok(h) => println!("{:?}", h),
        Err(e) => println!("{}", e),
    }
}
