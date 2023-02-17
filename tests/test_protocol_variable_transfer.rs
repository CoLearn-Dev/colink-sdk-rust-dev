#![allow(unused_variables)]
mod common;
use colink::{CoLink, Participant, ProtocolEntry};
use common::*;

struct Initiator;
#[colink::async_trait]
impl ProtocolEntry for Initiator {
    async fn start(
        &self,
        cl: CoLink,
        param: Vec<u8>,
        participants: Vec<Participant>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        for i in 0..8 {
            let key = &format!("output{}", i);
            let key2 = &format!("output_remote_storage{}", i);
            cl.send_variable(key, &param, &participants[1..participants.len()])
                .await?;
            cl.send_variable_with_remote_storage(
                key2,
                &param,
                &participants[1..participants.len()],
            )
            .await?;
        }
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
        for i in 0..8 {
            let key = &format!("output{}", i);
            let key2 = &format!("output_remote_storage{}", i);
            let msg = cl.receive_variable(key, &participants[0]).await?;
            cl.create_entry(&format!("tasks:{}:output{}", cl.get_task_id()?, i), &msg)
                .await?;
            let msg = cl
                .receive_variable_with_remote_storage(key2, &participants[0])
                .await?;
            cl.create_entry(
                &format!("tasks:{}:output_remote_storage{}", cl.get_task_id()?, i),
                &msg,
            )
            .await?;
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_vt() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (ir, iss, cls) = set_up_test_env(8).await?;
    for cl in &cls {
        colink::protocol_attach!(
            cl,
            ("variable_transfer_test:initiator", Initiator),
            ("variable_transfer_test:receiver", Receiver)
        );
    }
    let mut participants = vec![Participant {
        user_id: cls[0].get_user_id()?,
        role: "initiator".to_string(),
    }];
    for i in 1..8 {
        participants.push(Participant {
            user_id: cls[i].get_user_id()?,
            role: "receiver".to_string(),
        });
    }
    let data = "test".as_bytes();
    let task_id = cls[0]
        .run_task("variable_transfer_test", data, &participants, true)
        .await?;
    for idx in 1..8 {
        for idx2 in 0..8 {
            let res = cls[idx]
                .read_or_wait(&format!("tasks:{}:output{}", task_id, idx2))
                .await?;
            println!("{}", String::from_utf8_lossy(&res));
            assert!(res == data);
            let res = cls[idx]
                .read_or_wait(&format!("tasks:{}:output_remote_storage{}", task_id, idx2))
                .await?;
            println!("{}", String::from_utf8_lossy(&res));
            assert!(res == data);
        }
    }
    Ok(())
}
