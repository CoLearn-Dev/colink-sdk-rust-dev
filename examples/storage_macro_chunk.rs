use colink::{decode_jwt_without_validation, CoLink};
use rand::Rng;
use std::env;

const CHUNK_SIZE: usize = 1024 * 1024; // use 1MB chunks

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];
    let length = if args.len() > 2 {
        args[2].parse::<usize>().unwrap()
    } else {
        5e6 as usize // default to 5 * 10^6 bytes
    };
    let user_id = decode_jwt_without_validation(jwt).unwrap().user_id;
    println!("user_id: {}", user_id);
    let cl = CoLink::new(addr, jwt);
    let key_name = "storage_macro_demo:$chunk";
    let payload = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(length)
        .collect::<Vec<u8>>();

    // create
    println!("Creating entry...");
    let response = cl.create_entry(key_name, &payload.clone()).await?;
    println!("created entry at key name: {}", response);

    // read
    println!("Reading entry...");
    let data = cl.read_entry(key_name).await?;
    assert_eq!(data, payload);
    println!(
        "Read payload of {}MB ({} bytes), verified to be same as bytes written",
        payload.len() as f32 / CHUNK_SIZE as f32,
        payload.len()
    );

    // update
    println!("Updating entry...");
    let new_payload = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(length / 2)
        .collect::<Vec<u8>>();
    let response = cl.update_entry(key_name, &new_payload.clone()).await?;
    println!("updated entry at key name: {}", response);

    // read again to verify
    println!("Reading entry again...");
    let data = cl.read_entry(key_name).await?;
    assert_eq!(data, new_payload);
    println!(
        "Read payload of {}MB ({} bytes), verified to be same as the updated payload bytes",
        new_payload.len() as f32 / CHUNK_SIZE as f32,
        new_payload.len()
    );

    // delete
    println!("Deleting entry...");
    cl.delete_entry(key_name).await?;
    Ok(())
}
