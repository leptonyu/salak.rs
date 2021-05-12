use salak::*;

#[derive(FromEnvironment, Debug)]
pub struct DatabaseConfigObj {
    hello: String,
    world: Option<String>,
}
#[derive(FromEnvironment, Debug)]
pub struct DatabaseConfigDetail {
    #[salak(default = "str")]
    option_str: String,
    #[salak(default = 1)]
    option_i64: i64,
    option_arr: Vec<i64>,
    option_obj: Vec<DatabaseConfigObj>,
}

#[derive(FromEnvironment, Debug)]
#[salak(prefix = "database")]
pub struct DatabaseConfig {
    url: String,
    #[salak(default = "${database.username}")]
    name: String,
    #[salak(default = "salak")]
    username: String,
    password: Option<String>,
    description: String,
    #[salak(name = "ssl")]
    detail: DatabaseConfigDetail,
}

#[derive(FromEnvironment, Debug)]
pub enum Hello {
    OK,
    ERR,
}

fn main() {
    env_logger::init();
    let env = PropertyRegistry::new()
        .with_default_args(auto_read_sys_args_param!())
        .set_property("database.url", "localhost:5432")
        .set_property("database.description", "\\$\\{Hello\\}")
        .set_property("database.ssl.option_arr[0]", "10")
        .add_default::<DatabaseConfig>()
        .add_default_source(inline_toml!("app.toml"))
        .build();

    match env.load_config::<DatabaseConfig>() {
        Ok(h) => println!("{:?}", h),
        Err(e) => println!("{}", e),
    }

    for x in env.find_keys("") {
        println!("{}", &x);
    }
}
