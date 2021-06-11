use salak::*;
use salak_factory::postgresql::PostgresPool;

fn main() -> Result<(), PropertyError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()?;
    let env = Salak::builder()
        .register_default_resource::<PostgresPool>()?
        .configure_args(app_info!())
        .build()?;
    let _service = env.get_resource::<PostgresPool>()?;
    let _conn = _service.get()?;
    Ok(())
}
