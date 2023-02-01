use colink::extensions::instant_server::{InstantRegistry, InstantServer};
use std::{
    sync::{Arc, Mutex},
    thread,
};

#[tokio::test]
async fn test_counter_with_lock() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let _ir = InstantRegistry::new();
    let is = InstantServer::new();
    let cl = is.get_colink().switch_to_generated_user().await?;

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
    for i in 0..test_num {
        let ans_ = ans.lock().unwrap();
        assert!(ans_[i as usize] == i);
    }

    Ok(())
}
