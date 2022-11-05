use colink::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let ca_certificate = &args[1];
    let client_cert = &args[2];
    let client_key = &args[3];

    let cl = CoLink::new(addr, "")
        .ca_certificate(ca_certificate)
        .identity(client_cert, client_key);
    let core_pub_key = cl.request_info().await?.core_public_key;
    println!("{}", core_pub_key);

    Ok(())
}
