#![allow(unused_variables)]
use colink::{
    extensions::instant_server::{InstantServer, LocalRegistry},
    CoLink, Participant, ProtocolEntry,
};

struct Initiator;
#[colink::async_trait]
impl ProtocolEntry for Initiator {
    async fn start(
        &self,
        cl: CoLink,
        param: Vec<u8>,
        participants: Vec<Participant>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!("initiator");
        Ok(())
    }
}

struct Receiver;
#[colink::async_trait]
impl ProtocolEntry for Receiver {
    async fn start(
        &self,
        cl: CoLink,
        param: Vec<u8>,
        participants: Vec<Participant>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!("{}", String::from_utf8_lossy(&param));
        cl.create_entry(&format!("tasks:{}:output", cl.get_task_id()?), &param)
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let lr = LocalRegistry::new().await;
    let is0 = InstantServer::new();
    let is1 = InstantServer::new();
    let cl0 = is0.get_colink().switch_to_generated_user().await?;
    let cl1 = is1.get_colink().switch_to_generated_user().await?;
    colink::protocol_attach!(
        cl0,
        ("greetings:initiator", Initiator),
        ("greetings:receiver", Receiver)
    );
    colink::protocol_attach!(
        cl1,
        ("greetings:initiator", Initiator),
        ("greetings:receiver", Receiver)
    );
    let participants = vec![
        Participant {
            user_id: cl0.get_user_id()?,
            role: "initiator".to_string(),
        },
        Participant {
            user_id: cl1.get_user_id()?,
            role: "receiver".to_string(),
        },
    ];
    let task_id = cl0
        .run_task("greetings", "test".as_bytes(), &participants, true)
        .await?;
    let res = cl1
        .read_or_wait(&format!("tasks:{}:output", task_id))
        .await?;
    println!("{}", String::from_utf8_lossy(&res));
    Ok(())
}
