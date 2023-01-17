use crate::{application::*, utils::get_path_timestamp};
pub use async_trait::async_trait;
use clap::Parser;
use futures_lite::stream::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicQosOptions},
    types::FieldTable,
    Connection, ConnectionProperties, Consumer,
};
use prost::Message;
use rand::Rng;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread,
};
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
    vt_public_addr: Option<String>,
}

impl CoLinkProtocol {
    pub fn new(
        protocol_and_role: &str,
        cl: CoLink,
        user_func: Box<dyn ProtocolEntry>,
        vt_public_addr: Option<String>,
    ) -> Self {
        Self {
            protocol_and_role: protocol_and_role.to_string(),
            cl,
            user_func,
            vt_public_addr,
        }
    }

    pub async fn start(&self) -> Result<(), Error> {
        let mut consumer = self.get_mq_consumer().await?;
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
                        self.process_task(task).await?;
                    }
                    Err(e) => error!("Pull Task Error: {}.", e),
                }
            }
            delivery.ack(BasicAckOptions::default()).await.unwrap();
        }

        Ok(())
    }

    async fn process_task(&self, task: Task) -> Result<(), Error> {
        if task.status == "started" {
            let mut cl = self.cl.clone();
            cl.set_task_id(&task.task_id);
            #[cfg(feature = "variable_transfer")]
            {
                cl.vt_p2p_ctx =
                    Arc::new(crate::extensions::variable_transfer::p2p_inbox::VtP2pCtx {
                        public_addr: self.vt_public_addr.clone(),
                        ..Default::default()
                    });
            }
            let cl_clone = cl.clone();
            match self
                .user_func
                .start(cl, task.protocol_param, task.participants)
                .await
            {
                Ok(_) => {}
                Err(e) => error!("Task {}: {}.", task.task_id, e),
            }
            if cl_clone.vt_p2p_ctx.inbox_server.write().await.is_some() {
                let inbox_server = cl_clone.vt_p2p_ctx.inbox_server.write().await;
                inbox_server
                    .as_ref()
                    .unwrap()
                    .shutdown_channel
                    .send(())
                    .await?;
            }
            self.cl.finish_task(&task.task_id).await?;
        }
        Ok(())
    }

    async fn get_mq_consumer(&self) -> Result<Consumer, Error> {
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
                            get_path_timestamp(&list_entry.key_path)
                        } else {
                            list.task_ids_with_key_paths
                                .iter()
                                .map(|x| get_path_timestamp(&x.key_path))
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

        let mq_addr = self.cl.request_info().await?.mq_uri;
        let mq = Connection::connect(&mq_addr, ConnectionProperties::default()).await?;
        let channel = mq.create_channel().await?;
        channel.basic_qos(1, BasicQosOptions::default()).await?;
        let consumer = channel
            .basic_consume(
                &queue_name,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        Ok(consumer)
    }
}

pub fn _protocol_start(
    cl: CoLink,
    user_funcs: HashMap<String, Box<dyn ProtocolEntry + Send + Sync>>,
    keep_alive_when_disconnect: bool,
    vt_public_addr: Option<String>,
) -> Result<(), Error> {
    let mut operator_funcs: HashMap<String, Box<dyn ProtocolEntry + Send + Sync>> = HashMap::new();
    let mut protocols = HashSet::new();
    let failed_protocols = Arc::new(Mutex::new(HashSet::new()));
    for (protocol_and_role, user_func) in user_funcs {
        let cl = cl.clone();
        let failed_protocols = failed_protocols.clone();
        if protocol_and_role.ends_with(":@init") {
            let protocol_name = protocol_and_role[..protocol_and_role.len() - 6].to_string();
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let is_initialized_key =
                        format!("_internal:protocols:{}:_is_initialized", protocol_name);
                    let lock = cl.lock(&is_initialized_key).await?;
                    let res = cl.read_entry(&is_initialized_key).await;
                    if res.is_err() || res.unwrap()[0] == 0 {
                        let cl_clone = cl.clone();
                        match user_func
                            .start(cl_clone, Default::default(), Default::default())
                            .await
                        {
                            Ok(_) => {
                                cl.update_entry(&is_initialized_key, &[1]).await?;
                            }
                            Err(e) => {
                                error!("{}: {}.", protocol_and_role, e);
                                failed_protocols.lock().unwrap().insert(protocol_name);
                            }
                        }
                    }
                    cl.unlock(lock).await?;
                    Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
                })?;
        } else {
            protocols
                .insert(protocol_and_role[..protocol_and_role.rfind(':').unwrap()].to_string());
            operator_funcs.insert(protocol_and_role, user_func);
        }
    }
    for failed_protocol in &*failed_protocols.lock().unwrap() {
        protocols.remove(failed_protocol);
    }
    let cl_clone = cl.clone();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            for protocol_name in protocols {
                let is_initialized_key =
                    format!("_internal:protocols:{}:_is_initialized", protocol_name);
                cl_clone.update_entry(&is_initialized_key, &[1]).await?;
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
        })?;
    let mut threads = vec![];
    for (protocol_and_role, user_func) in operator_funcs {
        let cl = cl.clone();
        let vt_public_addr = vt_public_addr.clone();
        threads.push(thread::spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    match CoLinkProtocol::new(&protocol_and_role, cl, user_func, vt_public_addr)
                        .start()
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => error!("Protocol {}: {}.", protocol_and_role, e),
                    }
                });
        }));
    }
    if keep_alive_when_disconnect {
        for thread in threads {
            thread.join().unwrap();
        }
    } else {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let mut counter = 0;
                loop {
                    match cl.request_info().await {
                        Ok(_) => {
                            counter = 0;
                        }
                        Err(_) => {
                            counter += 1;
                            if counter >= 3 {
                                break;
                            }
                        }
                    }
                    let st = rand::thread_rng().gen_range(32..64);
                    tokio::time::sleep(tokio::time::Duration::from_secs(st)).await;
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
            })?;
    }
    Ok(())
}

#[derive(Debug, Parser)]
#[command(name = "CoLink-SDK", about = "CoLink-SDK")]
pub struct CommandLineArgs {
    /// Address of CoLink server
    #[arg(short, long, env = "COLINK_CORE_ADDR")]
    pub addr: String,

    /// User JWT
    #[arg(short, long, env = "COLINK_JWT")]
    pub jwt: String,

    /// Path to CA certificate.
    #[arg(long, env = "COLINK_CA_CERT")]
    pub ca: Option<String>,

    /// Path to client certificate.
    #[arg(long, env = "COLINK_CLIENT_CERT")]
    pub cert: Option<String>,

    /// Path to private key.
    #[arg(long, env = "COLINK_CLIENT_KEY")]
    pub key: Option<String>,

    /// Keep alive when disconnect.
    #[arg(long, env = "COLINK_KEEP_ALIVE_WHEN_DISCONNECT")]
    pub keep_alive_when_disconnect: bool,

    /// Public address for the variable transfer inbox.
    #[arg(long, env = "COLINK_VT_PUBLIC_ADDR")]
    pub vt_public_addr: Option<String>,
}

pub fn _colink_parse_args() -> (CoLink, bool, Option<String>) {
    tracing_subscriber::fmt::init();
    let CommandLineArgs {
        addr,
        jwt,
        ca,
        cert,
        key,
        keep_alive_when_disconnect,
        vt_public_addr,
    } = CommandLineArgs::parse();
    let mut cl = CoLink::new(&addr, &jwt);
    if let Some(ca) = ca {
        cl = cl.ca_certificate(&ca);
    }
    if let (Some(cert), Some(key)) = (cert, key) {
        cl = cl.identity(&cert, &key);
    }
    (cl, keep_alive_when_disconnect, vt_public_addr)
}

#[macro_export]
macro_rules! protocol_start {
    ( $( $x:expr ),* ) => {
        fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let (cl, keep_alive_when_disconnect, vt_public_addr) = colink::_colink_parse_args();

            let mut user_funcs: std::collections::HashMap<
                String,
                Box<dyn colink::ProtocolEntry + Send + Sync>,
            > = std::collections::HashMap::new();
            $(
                user_funcs.insert($x.0.to_string(), Box::new($x.1));
            )*

            colink::_protocol_start(cl, user_funcs, keep_alive_when_disconnect, vt_public_addr)?;

            Ok(())
        }
    };
}

#[macro_export]
macro_rules! protocol_attach {
    ( $cl:expr, $( $x:expr ),* ) => {
        {
            let cl = $cl.clone();
            let mut user_funcs: std::collections::HashMap<
                String,
                Box<dyn colink::ProtocolEntry + Send + Sync>,
            > = std::collections::HashMap::new();
            $(
                user_funcs.insert($x.0.to_string(), Box::new($x.1));
            )*
            std::thread::spawn(|| {
                colink::_protocol_start(cl, user_funcs, false, Some("127.0.0.1".to_string()))?;
                Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
            });
        }
    };
}
