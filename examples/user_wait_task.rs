use colink::{
    decode_jwt_without_validation, extensions::registry::UserRecord, CoLink, Participant,
};
use prost::Message;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];
    let target_user = &args[2];
    let user_id = decode_jwt_without_validation(jwt).unwrap().user_id;

    let user = UserRecord {
        user_id: target_user.to_string(),
        ..Default::default()
    };
    let mut payload = vec![];
    user.encode(&mut payload).unwrap();
    let cl = CoLink::new(addr, jwt);
    let participants = vec![Participant {
        user_id: user_id.to_string(),
        role: "query_from_registries".to_string(),
    }];
    let task_id = cl
        .run_task("registry", &payload, &participants, false)
        .await?;
    println!(
        "Task {} has been created, waiting for it to finish...",
        task_id
    );
    cl.wait_task(&task_id).await?;
    println!("Task {} finished", task_id);

    Ok(())
}
