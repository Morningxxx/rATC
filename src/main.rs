mod converter;
mod error;
mod model;
mod store;
mod subscription;

fn main() -> error::Result<()> {
    println!("rATC scaffold ok");
    Ok(())
}
