use colink::{
    extensions::instant_server::{InstantRegistry, InstantServer},
    Participant,
};
use rand::Rng;

const USER_NUM: [usize; 11] = [2, 2, 2, 2, 2, 3, 3, 4, 4, 5, 5];

#[tokio::test]
async fn test_greetings() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    tracing_subscriber::fmt::init();
    for i in 0..11 {
        test_main(USER_NUM[i as usize]).await?;
    }
    Ok(())
}

async fn test_main(
    user_num: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let _ir = InstantRegistry::new().await;
    let mut iss = vec![];
    let mut cls = vec![];
    for _ in 0..user_num {
        let is = InstantServer::new();
        let cl = is.get_colink().switch_to_generated_user().await?;
        iss.push(is);
        cls.push(cl);
    }
    for i in 0..user_num {
        let num: usize = rand::thread_rng().gen_range(1..4); // Generate the number of operators for testing multiple protocol operators.
        for _ in 0..num {
            {
                let cl = cls[i].clone();
                let time: u64 = rand::thread_rng().gen_range(0..1000);
                std::thread::spawn(move || {
                    std::thread::sleep(core::time::Duration::from_millis(time));
                    tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(async move {
                            cl.start_protocol_operator("greetings", &cl.get_user_id()?, false)
                                .await
                        })
                })
            };
        }
    }

    let random_number = rand::thread_rng().gen_range(0..1000);
    let mut participants = vec![Participant {
        user_id: cls[0].get_user_id()?,
        role: "initiator".to_string(),
    }];
    for i in 1..user_num {
        participants.push(Participant {
            user_id: cls[i].get_user_id()?,
            role: "receiver".to_string(),
        });
    }
    let data = random_number.to_string();
    let task_id = cls[0]
        .run_task("greetings", data.as_bytes(), &participants, true)
        .await?;
    for idx in 1..user_num {
        let res = cls[idx]
            .read_or_wait(&format!("tasks:{}:output", task_id))
            .await?;
        let msg = String::from_utf8_lossy(&res).to_string();
        println!("msg:{}", msg);
        assert!(msg.parse::<i32>()? == random_number);
    }
    Ok(())
}
