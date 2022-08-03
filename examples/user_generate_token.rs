use colink_sdk::CoLink;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];

    let mut cl = CoLink::new(addr, jwt);
    let new_jwt = cl.generate_token("user").await?;
    cl.update_jwt(&new_jwt)?;
    let guest_jwt = cl.generate_token("guest").await?;
    println!("{}\n{}", new_jwt, guest_jwt);

    Ok(())
}
