use salak::*;

fn main() -> Result<(), PropertyError> {
    let _ = Salak::builder().enable_args(app_info!()).build()?;
    Ok(())
}
