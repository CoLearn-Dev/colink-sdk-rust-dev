#![allow(unused_variables)]
use colink::{CoLink, Participant, ProtocolEntry};

struct Init;
#[colink::async_trait]
impl ProtocolEntry for Init {
    async fn start(
        &self,
        cl: CoLink,
        _param: Vec<u8>,                 // For init function, param is empty
        _participants: Vec<Participant>, // For init function, participants is empty
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Init
        Ok(())
    }
}

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

colink::protocol_start!(
    ("greetings:@init", Init),          // bind init function
    ("greetings:initiator", Initiator), // bind initiator's entry function
    ("greetings:receiver", Receiver)    // bind receiver's entry function
);
