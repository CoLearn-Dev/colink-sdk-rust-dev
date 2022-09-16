use colink::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];
    let expiration_timestamp: i64 = if args.len() > 2 {
        args[2].parse().unwrap()
    } else {
        // A default expiration timestamp at 31 days later
        chrono::Utc::now().timestamp() + 86400 * 31
    };

    let cl = CoLink::new(addr, jwt);
    let (pk, sk) = generate_user();
    let (_, core_pub_key) = cl.request_core_info().await?;
    let (signature_timestamp, sig) =
        prepare_import_user_signature(&pk, &sk, &core_pub_key, expiration_timestamp);
    println!(
        "{}",
        cl.import_user(&pk, signature_timestamp, expiration_timestamp, &sig)
            .await?
    );

    Ok(())
}
