use colink::CoLink;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];
    let instance_id = &args[2];
    let cl = CoLink::new(addr, jwt);
    cl.stop_protocol_operator(instance_id).await?;

    Ok(())
}
