use crate::colink_proto::*;
use core::task::{Context, Poll};
use futures_lite::ready;
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, Server, StatusCode};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use rand::{Rng, RngCore};
use rcgen::generate_simple_self_signed;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{Mutex, RwLock};
use tokio_rustls::rustls;
use tokio_rustls::rustls::ServerConfig;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub(crate) struct MyVTInbox {
    port: u16,
    jwt_secret: [u8; 32],
    #[allow(clippy::type_complexity)]
    data_map: Arc<RwLock<HashMap<(String, String), Vec<u8>>>>,
    pub(crate) shutdown_channel: tokio::sync::mpsc::Sender<()>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct VTInboxAuthContent {
    pub user_id: String,
}

impl MyVTInbox {
    fn new() -> Self {
        let mut jwt_secret: [u8; 32] = [0; 32];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut jwt_secret);
        let mut port = rand::thread_rng().gen_range(10000..30000);
        while std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            port = rand::thread_rng().gen_range(10000..30000);
        }
        let addr = ([0, 0, 0, 0], port).into();
        let data = Arc::new(RwLock::new(HashMap::new()));
        let data_clone = data.clone();
        // tls
        let (pub_cert, priv_key) = gen_cert();
        let tls_cfg = Arc::new(
            rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(
                    vec![rustls::Certificate(pub_cert)],
                    rustls::PrivateKey(priv_key),
                )
                .unwrap(),
        );
        // http server
        let service = make_service_fn(move |_| {
            let data = data.clone();
            async move {
                Ok::<_, Error>(service_fn(move |req| {
                    let data = data.clone();
                    async move {
                        let user_id = req.headers().get("user_id").unwrap().to_str()?.to_string(); // TODO unwrap
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
                        data.write().await.insert((user_id, key), body.to_vec());
                        Ok::<_, Error>(Response::default())
                    }
                }))
            }
        });
        let incoming = AddrIncoming::bind(&addr).unwrap();
        let server = Server::builder(TlsAcceptor::new(tls_cfg, incoming)).serve(service);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
        let graceful = server.with_graceful_shutdown(async move {
            rx.recv().await;
        });
        tokio::spawn(async { graceful.await });
        Self {
            port,
            jwt_secret,
            data_map: data_clone,
            shutdown_channel: tx,
        }
    }
}

fn gen_cert() -> (Vec<u8>, Vec<u8>) {
    let subject_alt_names: &[_] = &["vt-p2p.colink".to_string()];
    let cert = generate_simple_self_signed(subject_alt_names).unwrap();
    let pub_cert = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    (pub_cert, priv_key)
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
                        .header("token", &remote_inbox.vt_jwt)
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
                        "http://{}:{}",
                        self.vt_p2p.my_public_addr.as_ref().unwrap(),
                        self.vt_p2p.my_inbox.read().await.as_ref().unwrap().port
                    ),
                    vt_jwt,
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
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // TODO channel
        }
    }
}

// https://github.com/rustls/hyper-rustls/blob/21c4d3749c5bac74eb934c56f1f92d76e1b88554/examples/server.rs#L70-L173
// BEGIN hyper-rustls
enum State {
    Handshaking(tokio_rustls::Accept<AddrStream>),
    Streaming(tokio_rustls::server::TlsStream<AddrStream>),
}

// tokio_rustls::server::TlsStream doesn't expose constructor methods,
// so we have to TlsAcceptor::accept and handshake to have access to it
// TlsStream implements AsyncRead/AsyncWrite handshaking tokio_rustls::Accept first
pub struct TlsStream {
    state: State,
}

impl TlsStream {
    fn new(stream: AddrStream, config: Arc<ServerConfig>) -> TlsStream {
        let accept = tokio_rustls::TlsAcceptor::from(config).accept(stream);
        TlsStream {
            state: State::Handshaking(accept),
        }
    }
}

impl AsyncRead for TlsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<std::io::Result<()>> {
        let pin = self.get_mut();
        match pin.state {
            State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
                Ok(mut stream) => {
                    let result = Pin::new(&mut stream).poll_read(cx, buf);
                    pin.state = State::Streaming(stream);
                    result
                }
                Err(err) => Poll::Ready(Err(err)),
            },
            State::Streaming(ref mut stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let pin = self.get_mut();
        match pin.state {
            State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
                Ok(mut stream) => {
                    let result = Pin::new(&mut stream).poll_write(cx, buf);
                    pin.state = State::Streaming(stream);
                    result
                }
                Err(err) => Poll::Ready(Err(err)),
            },
            State::Streaming(ref mut stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.state {
            State::Handshaking(_) => Poll::Ready(Ok(())),
            State::Streaming(ref mut stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.state {
            State::Handshaking(_) => Poll::Ready(Ok(())),
            State::Streaming(ref mut stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

pub struct TlsAcceptor {
    config: Arc<ServerConfig>,
    incoming: AddrIncoming,
}

impl TlsAcceptor {
    pub fn new(config: Arc<ServerConfig>, incoming: AddrIncoming) -> TlsAcceptor {
        TlsAcceptor { config, incoming }
    }
}

impl Accept for TlsAcceptor {
    type Conn = TlsStream;
    type Error = std::io::Error;

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        let pin = self.get_mut();
        match ready!(Pin::new(&mut pin.incoming).poll_accept(cx)) {
            Some(Ok(sock)) => Poll::Ready(Some(Ok(TlsStream::new(sock, pin.config.clone())))),
            Some(Err(e)) => Poll::Ready(Some(Err(e))),
            None => Poll::Ready(None),
        }
    }
}
// END hyper-rustls
