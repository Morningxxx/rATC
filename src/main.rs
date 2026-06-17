mod config;
mod converter;
mod error;
mod model;
mod store;
mod subscription;
mod sysproxy;
mod xray;

fn main() -> error::Result<()> {
    println!("rATC scaffold ok");
    Ok(())
}
