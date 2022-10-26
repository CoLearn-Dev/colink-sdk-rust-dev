use colink::extensions::registry::{Registries, Registry};
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
    let (pk, sk) = generate_user();
    let (_, core_pub_key, _) = cl.request_info().await?;
    let (signature_timestamp, sig) =
        prepare_import_user_signature(&pk, &sk, &core_pub_key, expiration_timestamp);
    let registry_user = cl
        .import_user(&pk, signature_timestamp, expiration_timestamp, &sig)
        .await?;
    println!("registry_user:");
    println!("{}", registry_user);
    let clt = CoLink::new(addr, &registry_user);
    let registry_jwt = clt
        .generate_token_with_expiration_time(expiration_timestamp, "guest")
        .await?;

    let registry = Registry {
        address: addr.clone(),
        guest_jwt: registry_jwt,
    };
    let registries = Registries {
        registries: vec![registry],
    };
    clt.update_registries(&registries).await?;
    for i in 0..num {
        let (pk, sk) = generate_user();
        let (_, core_pub_key, _) = cl.request_info().await?;
        let (signature_timestamp, sig) =
            prepare_import_user_signature(&pk, &sk, &core_pub_key, expiration_timestamp);
        users.push(
            cl.import_user(&pk, signature_timestamp, expiration_timestamp, &sig)
                .await?,
        );
        let cl = CoLink::new(addr, &users[i]);
        cl.update_registries(&registries).await?;
    }
    println!("user:");
    for i in 0..num {
        println!("{}", users[i]);
    }

    Ok(())
}
