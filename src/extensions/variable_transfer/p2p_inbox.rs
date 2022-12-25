use crate::colink_proto::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, Server, StatusCode};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub(crate) struct MyVTInbox {
    port: u16,
    #[allow(clippy::type_complexity)]
    data_map: Arc<RwLock<HashMap<(String, String), Vec<u8>>>>,
    pub(crate) shutdown_channel: tokio::sync::mpsc::Sender<()>,
}

impl MyVTInbox {
    fn new() -> Self {
        let mut port = rand::thread_rng().gen_range(10000..30000);
        while std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            port = rand::thread_rng().gen_range(10000..30000);
        }
        let addr = ([0, 0, 0, 0], port).into();
        let data = Arc::new(RwLock::new(HashMap::new()));
        let data_clone = data.clone();
        let service = make_service_fn(move |_| {
            let data = data.clone();
            async move {
                Ok::<_, Error>(service_fn(move |req| {
                    let data = data.clone();
                    async move {
                        let user_id = req.headers().get("user_id").unwrap().to_str()?.to_string(); // TODO unwrap
                        let key = req.headers().get("key").unwrap().to_str()?.to_string();
                        let body = hyper::body::to_bytes(req.into_body()).await?;
                        data.write().await.insert((user_id, key), body.to_vec());
                        Ok::<_, Error>(Response::new(Body::default()))
                    }
                }))
            }
        });
        let server = Server::bind(&addr).serve(service);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
        let graceful = server.with_graceful_shutdown(async move {
            rx.recv().await;
        });
        tokio::spawn(async { graceful.await });
        Self {
            port,
            data_map: data_clone,
            shutdown_channel: tx,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct VTInbox {
    addr: String,
    vt_jwt: String,
}

#[derive(Default)]
pub(crate) struct VTP2P {
    pub(crate) my_public_addr: Option<String>,
    pub(crate) has_created_inbox: Mutex<bool>,
    pub(crate) my_inbox: RwLock<Option<MyVTInbox>>,
    pub(crate) has_configured_inbox: RwLock<HashSet<String>>,
    pub(crate) remote_inboxes: RwLock<HashMap<String, Option<VTInbox>>>,
}

impl crate::application::CoLink {
    pub(crate) async fn _set_variable_p2p(
        &self,
        key: &str,
        payload: &[u8],
        receivers: &[Participant],
    ) -> Result<(), Error> {
        for receiver in receivers {
            if !self
                .vt_p2p
                .remote_inboxes
                .read()
                .await
                .contains_key(&receiver.user_id)
            {
                let inbox = self._get_variable_remote_storage("inbox", receiver).await?;
                let inbox: VTInbox = serde_json::from_slice(&inbox)?;
                let inbox = if inbox.addr.is_empty() {
                    None
                } else {
                    Some(inbox)
                };
                self.vt_p2p
                    .remote_inboxes
                    .write()
                    .await
                    .insert(receiver.user_id.clone(), inbox);
            }
            match self
                .vt_p2p
                .remote_inboxes
                .read()
                .await
                .get(&receiver.user_id)
                .unwrap()
            {
                Some(remote_inbox) => {
                    let req = Request::builder()
                        .method(Method::POST)
                        .uri(&remote_inbox.addr)
                        .header("user_id", self.get_user_id()?)
                        .header("key", key)
                        .body(Body::from(payload.to_vec()))?;
                    let client = Client::new();
                    let resp = client.request(req).await?;
                    if resp.status() != StatusCode::OK {
                        Err(format!("Remote inbox: error {}", resp.status()))?;
                    }
                }
                None => Err("Remote inbox: not available")?,
            }
        }
        Ok(())
    }

    pub(crate) async fn _get_variable_p2p(
        &self,
        key: &str,
        sender: &Participant,
    ) -> Result<Vec<u8>, Error> {
        if !self
            .vt_p2p
            .has_configured_inbox
            .read()
            .await
            .contains(&sender.user_id)
        {
            if self.vt_p2p.my_public_addr.is_some()
                && !(*self.vt_p2p.has_created_inbox.lock().await)
            {
                let mut has_created_inbox = self.vt_p2p.has_created_inbox.lock().await;
                let my_inbox = MyVTInbox::new();
                *self.vt_p2p.my_inbox.write().await = Some(my_inbox);
                *has_created_inbox = true;
            }
            let vt_inbox = if self.vt_p2p.my_public_addr.is_none() {
                VTInbox {
                    addr: "".to_string(),
                    vt_jwt: "".to_string(),
                }
            } else {
                VTInbox {
                    addr: format!(
                        "http://{}:{}",
                        self.vt_p2p.my_public_addr.as_ref().unwrap(),
                        self.vt_p2p.my_inbox.read().await.as_ref().unwrap().port
                    ),
                    vt_jwt: "".to_string(), //TODO
                }
            };
            self._set_variable_remote_storage(
                "inbox",
                &serde_json::to_vec(&vt_inbox)?,
                &[sender.clone()],
            )
            .await?;
            self.vt_p2p
                .has_configured_inbox
                .write()
                .await
                .insert(sender.user_id.clone());
        }
        if self.vt_p2p.my_public_addr.is_none() {
            Err("Remote inbox: not available")?;
        }
        loop {
            let my_inbox = self.vt_p2p.my_inbox.read().await;
            let data_map = my_inbox.as_ref().unwrap().data_map.read().await;
            let data = data_map.get(&(sender.user_id.clone(), key.to_string()));
            if data.is_some() {
                return Ok(data.unwrap().clone());
            }
            drop(data_map);
            drop(my_inbox);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}
