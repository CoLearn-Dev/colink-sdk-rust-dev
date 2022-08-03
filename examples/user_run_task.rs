use colink_sdk::{decode_jwt_without_validation, CoLink, Participant};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt_a = &args[1];
    let jwt_b = &args[2];
    let msg = if args.len() > 3 { &args[3] } else { "hello" };
    let user_id_a = decode_jwt_without_validation(jwt_a).unwrap().user_id;
    let user_id_b = decode_jwt_without_validation(jwt_b).unwrap().user_id;

    let participants = vec![
        Participant {
            user_id: user_id_a.to_string(),
            role: "initiator".to_string(),
        },
        Participant {
            user_id: user_id_b.to_string(),
            role: "receiver".to_string(),
        },
    ];
    let cl = CoLink::new(addr, jwt_a);
    let task_id = cl
        .run_task("greetings", msg.as_bytes(), &participants, true)
        .await?;
    println!(
        "Task {} has been created, but it will remain in waiting status until the protocol starts.",
        task_id
    );

    Ok(())
}
