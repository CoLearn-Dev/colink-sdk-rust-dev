use crate::colink_proto::*;
use hyper::server::conn::AddrIncoming;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, Server, StatusCode};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_rustls::rustls::{self, RootCertStore};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub(crate) struct VTInboxServer {
    port: u16,
    jwt_secret: [u8; 32],
    tls_cert: Vec<u8>,
    #[allow(clippy::type_complexity)]
    data_map: Arc<RwLock<HashMap<(String, String), Vec<u8>>>>,
    pub(crate) shutdown_channel: tokio::sync::mpsc::Sender<()>,
    #[allow(clippy::type_complexity)]
    pub(crate) notification_channels:
        Arc<RwLock<HashMap<(String, String), tokio::sync::mpsc::Sender<()>>>>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct VTInboxAuthContent {
    pub user_id: String,
}

impl VTInboxServer {
    fn new() -> Self {
        let mut jwt_secret: [u8; 32] = [0; 32];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut jwt_secret);
        let data = Arc::new(RwLock::new(HashMap::new()));
        let data_clone = data.clone();
        #[allow(clippy::type_complexity)]
        let notification_channels: Arc<
            RwLock<HashMap<(String, String), tokio::sync::mpsc::Sender<()>>>,
        > = Arc::new(RwLock::new(HashMap::new()));
        let notification_channels_clone = notification_channels.clone();
        // tls
        let (tls_cert, priv_key) = super::tls_utils::gen_cert();
        let tls_cfg = Arc::new(
            rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(
                    vec![rustls::Certificate(tls_cert.clone())],
                    rustls::PrivateKey(priv_key),
                )
                .unwrap(),
        );
        // http server
        let service = make_service_fn(move |_| {
            let data = data.clone();
            let notification_channels = notification_channels.clone();
            async move {
                Ok::<_, Error>(service_fn(move |req| {
                    let data = data.clone();
                    let notification_channels = notification_channels.clone();
                    async move {
                        if !req.headers().contains_key("user_id")
                            || !req.headers().contains_key("key")
                            || !req.headers().contains_key("token")
                        {
                            let mut err: Response<Body> = Response::default();
                            *err.status_mut() = StatusCode::BAD_REQUEST;
                            return Ok(err);
                        }
                        let user_id = req.headers().get("user_id").unwrap().to_str()?.to_string();
                        let key = req.headers().get("key").unwrap().to_str()?.to_string();
                        let token = req.headers().get("token").unwrap().to_str()?.to_string();
                        // verify jwt
                        let token = match jsonwebtoken::decode::<VTInboxAuthContent>(
                            &token,
                            &DecodingKey::from_secret(&jwt_secret),
                            &Validation {
                                validate_exp: false,
                                ..Default::default()
                            },
                        ) {
                            Ok(token) => token,
                            Err(_) => {
                                let mut err: Response<Body> = Response::default();
                                *err.status_mut() = StatusCode::UNAUTHORIZED;
                                return Ok(err);
                            }
                        };
                        if token.claims.user_id != user_id {
                            let mut err: Response<Body> = Response::default();
                            *err.status_mut() = StatusCode::UNAUTHORIZED;
                            return Ok(err);
                        }
                        // payload
                        let body = hyper::body::to_bytes(req.into_body()).await?;
                        data.write()
                            .await
                            .insert((user_id.clone(), key.clone()), body.to_vec());
                        let notification_channels = notification_channels.read().await;
                        let nc = notification_channels.get(&(user_id, key));
                        if nc.is_some() {
                            nc.unwrap().send(()).await?;
                        }
                        drop(notification_channels);
                        Ok::<_, Error>(Response::default())
                    }
                }))
            }
        });
        let mut port = rand::thread_rng().gen_range(10000..30000);
        while std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            port = rand::thread_rng().gen_range(10000..30000);
        }
        let addr = ([0, 0, 0, 0], port).into();
        let incoming = AddrIncoming::bind(&addr).unwrap();
        let server =
            Server::builder(super::tls_utils::TlsAcceptor::new(tls_cfg, incoming)).serve(service);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
        let graceful = server.with_graceful_shutdown(async move {
            rx.recv().await;
        });
        tokio::spawn(async { graceful.await });
        Self {
            port,
            jwt_secret,
            tls_cert,
            data_map: data_clone,
            shutdown_channel: tx,
            notification_channels: notification_channels_clone,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct VTInbox {
    addr: String,
    vt_jwt: String,
    tls_cert: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct VTP2PCTX {
    pub(crate) my_public_addr: Option<String>,
    pub(crate) has_created_inbox: Mutex<bool>,
    pub(crate) my_inbox: RwLock<Option<VTInboxServer>>,
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
            // TODO parallel and fallback
            if !self
                .vt_p2p
                .remote_inboxes
                .read()
                .await
                .contains_key(&receiver.user_id)
            {
                let inbox = self.get_variable_with_remote_storage("inbox", receiver).await?;
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
                    let mut roots = RootCertStore::empty();
                    roots.add_parsable_certificates(&[remote_inbox.tls_cert.clone()]);
                    let tls_cfg = rustls::ClientConfig::builder()
                        .with_safe_defaults()
                        .with_root_certificates(roots)
                        .with_no_client_auth();
                    let https = hyper_rustls::HttpsConnectorBuilder::new()
                        .with_tls_config(tls_cfg)
                        .https_only()
                        .with_server_name(
                            super::tls_utils::SELF_SIGNED_CERT_DOMAIN_NAME.to_string(),
                        )
                        .enable_http1()
                        .build();
                    let client: Client<_, hyper::Body> = Client::builder().build(https);
                    let req = Request::builder()
                        .method(Method::POST)
                        .uri(&remote_inbox.addr)
                        .header("user_id", self.get_user_id()?)
                        .header("key", key)
                        .header("token", &remote_inbox.vt_jwt)
                        .body(Body::from(payload.to_vec()))?;
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
        // send inbox information to the sender by remote_storage
        if !self
            .vt_p2p
            .has_configured_inbox
            .read()
            .await
            .contains(&sender.user_id)
        {
            // create inbox if it does not exist
            if self.vt_p2p.my_public_addr.is_some()
                && !(*self.vt_p2p.has_created_inbox.lock().await)
            {
                let mut has_created_inbox = self.vt_p2p.has_created_inbox.lock().await;
                let my_inbox = VTInboxServer::new();
                *self.vt_p2p.my_inbox.write().await = Some(my_inbox);
                *has_created_inbox = true;
            }
            // generate vt_inbox information for the sender
            let vt_inbox = if self.vt_p2p.my_public_addr.is_none() {
                VTInbox {
                    addr: "".to_string(),
                    vt_jwt: "".to_string(),
                    tls_cert: Vec::new(),
                }
            } else {
                let jwt_secret = self
                    .vt_p2p
                    .my_inbox
                    .read()
                    .await
                    .as_ref()
                    .unwrap()
                    .jwt_secret;
                let vt_jwt = jsonwebtoken::encode(
                    &Header::default(),
                    &VTInboxAuthContent {
                        user_id: sender.user_id.clone(),
                    },
                    &EncodingKey::from_secret(&jwt_secret),
                )?;
                VTInbox {
                    addr: format!(
                        "https://{}:{}",
                        self.vt_p2p.my_public_addr.as_ref().unwrap(),
                        self.vt_p2p.my_inbox.read().await.as_ref().unwrap().port
                    ),
                    vt_jwt,
                    tls_cert: self
                        .vt_p2p
                        .my_inbox
                        .read()
                        .await
                        .as_ref()
                        .unwrap()
                        .tls_cert
                        .clone(),
                }
            };
            self.set_variable_with_remote_storage(
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
            let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
            my_inbox
                .as_ref()
                .unwrap()
                .notification_channels
                .write()
                .await
                .insert((sender.user_id.clone(), key.to_string()), tx);
            // try again after creating the channel
            let data_map = my_inbox.as_ref().unwrap().data_map.read().await;
            let data = data_map.get(&(sender.user_id.clone(), key.to_string()));
            if data.is_some() {
                return Ok(data.unwrap().clone());
            }
            drop(data_map);
            drop(my_inbox);
            rx.recv().await;
            let my_inbox = self.vt_p2p.my_inbox.read().await;
            let data_map = my_inbox.as_ref().unwrap().data_map.read().await;
            let data = data_map.get(&(sender.user_id.clone(), key.to_string()));
            if data.is_some() {
                return Ok(data.unwrap().clone());
            } else {
                Err("Fail to retrieve data from the inbox")?
            }
            drop(data_map);
            drop(my_inbox);
        }
    }
}
