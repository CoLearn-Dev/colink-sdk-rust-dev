use colink::{
    extensions::instant_server::{InstantRegistry, InstantServer},
    generate_user, prepare_import_user_signature, CoLink,
};
use std::{thread,sync::{Arc,Mutex}};

async fn user_lock_fn(cl:&CoLink) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    cl.update_entry("example_lock_counter", &0_i32.to_le_bytes())
        .await
        .unwrap();
    let test_num = 10;
    let mut ths = vec![];
    let ans = Arc::new(Mutex::new(vec![]));
    for _ in 0..test_num {
        let cl = cl.clone();
        ths.push(thread::spawn({
            let ans_clone = Arc::clone(&ans);
            move || {
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async move {
                        let lock = cl.lock("example_lock_name").await.unwrap();
                        let num = cl.read_entry("example_lock_counter").await.unwrap();
                        let num = i32::from_le_bytes(<[u8; 4]>::try_from(num).unwrap());
                        // println!("{}", num);
                        let mut ans_lock = ans_clone.lock().unwrap();
                        ans_lock.push(num);
                        cl.update_entry("example_lock_counter", &(num + 1).to_le_bytes())
                            .await
                            .unwrap();
                        cl.unlock(lock).await.unwrap();
                    });
            }
        }));
    }
    for th in ths {
        th.join().unwrap();
    }
    // println!("{:?}", ans);
    for i in 0..test_num {
        let ans_ = ans.lock().unwrap();
        assert!(ans_[i as usize] == i);
    }

    Ok(())
}

#[tokio::test]
async fn test_user_lock() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let _ir = InstantRegistry::new();
    let is = InstantServer::new();
    let cl = is.get_colink();
    let core_addr = cl.get_core_addr()?;

    let expiration_timestamp = chrono::Utc::now().timestamp() + 86400 * 31;
    let (pk, sk) = generate_user();
    let core_pub_key = cl.request_info().await?.core_public_key;
    let (signature_timestamp, sig) =
        prepare_import_user_signature(&pk, &sk, &core_pub_key, expiration_timestamp);

    let user_jwt = cl
        .import_user(&pk, signature_timestamp, expiration_timestamp, &sig)
        .await?;

    let cl=CoLink::new(&core_addr,&user_jwt);
    user_lock_fn(&cl).await?;

    Ok(())
}
