use colink_sdk::{decode_jwt_without_validation, CoLink, Participant};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt_initiator = &args[1];
    let msg = "hello";
    let user_id_initiator = decode_jwt_without_validation(jwt_initiator)
        .unwrap()
        .user_id;

    let mut participants = vec![Participant {
        user_id: user_id_initiator.to_string(),
        role: "initiator".to_string(),
    }];
    for i in 2..args.len() {
        participants.push(Participant {
            user_id: decode_jwt_without_validation(&args[i])
                .unwrap()
                .user_id
                .to_string(),
            role: "receiver".to_string(),
        });
    }

    let cl = CoLink::new(addr, jwt_initiator);
    let task_id = cl
        .run_task("greetings", msg.as_bytes(), &participants, true)
        .await?;
    println!(
        "Task {} has been created, but it will remain in waiting status until the protocol starts.",
        task_id
    );

    Ok(())
}
