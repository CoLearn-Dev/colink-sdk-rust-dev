#![allow(unused_variables)]
use colink_sdk::{CoLink, Participant, ProtocolEntry};

struct Initiator;
#[colink_sdk::async_trait]
impl ProtocolEntry for Initiator {
    async fn start(
        &self,
        cl: CoLink,
        param: Vec<u8>,
        participants: Vec<Participant>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!("initiator");
        cl.set_variable("output", "hello".as_bytes(), &[participants[0].clone()])
            .await?;
        Ok(())
    }
}

struct Receiver;
#[colink_sdk::async_trait]
impl ProtocolEntry for Receiver {
    async fn start(
        &self,
        cl: CoLink,
        param: Vec<u8>,
        participants: Vec<Participant>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let msg = cl.get_variable("output", &participants[0]).await?;
        println!("{}", String::from_utf8_lossy(&msg));
        cl.create_entry(&format!("tasks:{}:output", cl.get_task_id()?), &msg)
            .await?;
        Ok(())
    }
}

colink_sdk::protocol_start!(
    ("variable_transfer_example:initiator", Initiator),
    ("variable_transfer_example:receiver", Receiver)
);
