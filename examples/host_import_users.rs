use colink::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];
    let num = &args[2];
    let num: usize = num.parse().unwrap();
    let expiration_timestamp: i64 = if args.len() > 3 {
        args[3].parse().unwrap()
    } else {
        // A default expiration timestamp at 31 days later
        chrono::Utc::now().timestamp() + 86400 * 31
    };

    let cl = CoLink::new(addr, jwt);
    let mut users = vec![];
    for _ in 0..num {
        let (pk, sk) = generate_user();
        let core_pub_key = cl.request_info().await?.core_public_key;
        let (signature_timestamp, sig) =
            prepare_import_user_signature(&pk, &sk, &core_pub_key, expiration_timestamp);
        users.push(
            cl.import_user(&pk, signature_timestamp, expiration_timestamp, &sig)
                .await?,
        );
    }
    for i in 0..num {
        println!("{}", users[i]);
    }

    Ok(())
}
