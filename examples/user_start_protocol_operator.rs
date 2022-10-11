use colink::{decode_jwt_without_validation, CoLink};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];
    let protocol_name = &args[2];
    let user_id = decode_jwt_without_validation(jwt).unwrap().user_id;
    let cl = CoLink::new(addr, jwt);
    let instance_id = cl
        .start_protocol_operator(protocol_name, &user_id, true)
        .await?;
    println!("Instance id: {}", instance_id);

    Ok(())
}
