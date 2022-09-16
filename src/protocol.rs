use crate::application::*;
pub use async_trait::async_trait;
use futures_lite::stream::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicQosOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use prost::Message;
use std::{collections::HashMap, sync::mpsc::channel, thread};
use structopt::StructOpt;
use tracing::{debug, error};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[async_trait]
pub trait ProtocolEntry {
    async fn start(
        &self,
        cl: CoLink,
        param: Vec<u8>,
        participants: Vec<Participant>,
    ) -> Result<(), Error>;
}
pub struct CoLinkProtocol {
    protocol_and_role: String,
    cl: CoLink,
    user_func: Box<dyn ProtocolEntry>,
}

impl CoLinkProtocol {
    pub fn new(protocol_and_role: &str, cl: CoLink, user_func: Box<dyn ProtocolEntry>) -> Self {
        Self {
            protocol_and_role: protocol_and_role.to_string(),
            cl,
            user_func,
        }
    }

    pub async fn start(&self) -> Result<(), Error> {
        let operator_mq_key = format!("_internal:protocols:{}:operator_mq", self.protocol_and_role);
        let lock = self.cl.lock(&operator_mq_key).await?;
        let res = self
            .cl
            .read_entries(&[StorageEntry {
                key_name: operator_mq_key.clone(),
                ..Default::default()
            }])
            .await;
        let queue_name = match res {
            Ok(res) => {
                let operator_mq_entry = &res[0];
                String::from_utf8(operator_mq_entry.payload.clone()).unwrap()
            }
            Err(_) => {
                let list_key = format!("_internal:protocols:{}:started", self.protocol_and_role);
                let latest_key = format!(
                    "_internal:protocols:{}:started:latest",
                    self.protocol_and_role
                );
                let res = self
                    .cl
                    .read_entries(&[StorageEntry {
                        key_name: list_key,
                        ..Default::default()
                    }])
                    .await;
                let start_timestamp = match res {
                    Ok(res) => {
                        let list_entry = &res[0];
                        let list: CoLinkInternalTaskIdList =
                            Message::decode(&*list_entry.payload).unwrap();
                        if list.task_ids_with_key_paths.is_empty() {
                            get_timestamp(&list_entry.key_path)
                        } else {
                            list.task_ids_with_key_paths
                                .iter()
                                .map(|x| get_timestamp(&x.key_path))
                                .min()
                                .unwrap_or(i64::MAX)
                        }
                    }
                    Err(_) => 0i64,
                };
                let queue_name = self
                    .cl
                    .subscribe(&latest_key, Some(start_timestamp))
                    .await?;
                self.cl
                    .create_entry(&operator_mq_key, queue_name.as_bytes())
                    .await?;
                queue_name
            }
        };
        self.cl.unlock(lock).await?;

        let (mq_addr, _) = self.cl.request_core_info().await?;
        let mq = Connection::connect(&mq_addr, ConnectionProperties::default()).await?;
        let channel = mq.create_channel().await?;
        channel.basic_qos(1, BasicQosOptions::default()).await?;
        let mut consumer = channel
            .basic_consume(
                &queue_name,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.expect("error in consumer");
            let data = String::from_utf8_lossy(&delivery.data);
            debug!("Received [{}]", data);
            let message: SubscriptionMessage = prost::Message::decode(&*delivery.data).unwrap();
            if message.change_type != "delete" {
                let task_id: Task = prost::Message::decode(&*message.payload).unwrap();
                let res = self
                    .cl
                    .read_entries(&[StorageEntry {
                        key_name: format!("_internal:tasks:{}", task_id.task_id),
                        ..Default::default()
                    }])
                    .await;
                match res {
                    Ok(res) => {
                        let task_entry = &res[0];
                        let task: Task = prost::Message::decode(&*task_entry.payload).unwrap();
                        if task.status == "started" {
                            // begin user func
                            let mut cl = self.cl.clone();
                            cl.set_task_id(&task.task_id);
                            match self
                                .user_func
                                .start(cl, task.protocol_param, task.participants)
                                .await
                            {
                                Ok(_) => {}
                                Err(e) => error!("Task {}: {}.", task.task_id, e),
                            }
                            // end user func
                            self.cl.finish_task(&task.task_id).await?;
                        }
                    }
                    Err(e) => error!("Pull Task Error: {}.", e),
                }
            }
            delivery.ack(BasicAckOptions::default()).await.unwrap();
        }

        Ok(())
    }
}

fn get_timestamp(key_path: &str) -> i64 {
    let pos = key_path.rfind('@').unwrap();
    key_path[pos + 1..].parse().unwrap()
}

pub fn _protocol_start(
    cl: CoLink,
    user_funcs: HashMap<String, Box<dyn ProtocolEntry + Send + Sync>>,
) -> Result<(), Error> {
    for (protocol_and_role, user_func) in user_funcs {
        let cl = cl.clone();
        thread::spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    match CoLinkProtocol::new(&protocol_and_role, cl, user_func)
                        .start()
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => error!("Protocol {}: {}.", protocol_and_role, e),
                    }
                });
        });
    }
    let (tx, rx) = channel();
    ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
        .expect("Error setting Ctrl-C handler");
    println!("Started");
    rx.recv().expect("Could not receive from channel.");
    println!("Exiting...");
    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(name = "CoLink-SDK", about = "CoLink-SDK")]
pub struct CommandLineArgs {
    /// Address of CoLink server
    #[structopt(short, long)]
    pub addr: String,

    /// User JWT
    #[structopt(short, long)]
    pub jwt: String,

    /// Path to CA certificate.
    #[structopt(long)]
    pub ca: Option<String>,

    /// Path to client certificate.
    #[structopt(long)]
    pub cert: Option<String>,

    /// Path to private key.
    #[structopt(long)]
    pub key: Option<String>,
}

pub fn _colink_parse_args() -> CoLink {
    let CommandLineArgs {
        addr,
        jwt,
        ca,
        cert,
        key,
    } = CommandLineArgs::from_args();
    let mut cl = CoLink::new(&addr, &jwt);
    if let Some(ca) = ca {
        cl = cl.ca_certificate(&ca);
    }
    if let (Some(cert), Some(key)) = (cert, key) {
        cl = cl.identity(&cert, &key);
    }
    cl
}

#[macro_export]
macro_rules! protocol_start {
    ( $( $x:expr ),* ) => {
        fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let cl = colink::_colink_parse_args();

            let mut user_funcs: std::collections::HashMap<
                String,
                Box<dyn colink::ProtocolEntry + Send + Sync>,
            > = std::collections::HashMap::new();
            $(
                user_funcs.insert($x.0.to_string(), Box::new($x.1));
            )*

            colink::_protocol_start(cl, user_funcs)?;

            Ok(())
        }
    };
}
