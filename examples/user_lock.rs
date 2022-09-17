use colink::CoLink;
use std::{env, thread};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];

    let cl = CoLink::new(addr, jwt);
    cl.update_entry("example_lock_counter", &0_i32.to_le_bytes())
        .await
        .unwrap();
    let mut ths = vec![];
    for _ in 0..10 {
        let cl = cl.clone();
        ths.push(thread::spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let lock = cl.lock("example_lock_name").await.unwrap();
                    let num = cl.read_entry("example_lock_counter").await.unwrap();
                    let num = i32::from_le_bytes(<[u8; 4]>::try_from(num).unwrap());
                    println!("{}", num);
                    cl.update_entry("example_lock_counter", &(num + 1).to_le_bytes())
                        .await
                        .unwrap();
                    cl.unlock(lock).await.unwrap();
                });
        }));
    }
    for th in ths {
        th.join().unwrap();
    }

    Ok(())
}
