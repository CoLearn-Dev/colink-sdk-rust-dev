pub use crate::colink_proto::co_link_client::CoLinkClient;
pub use crate::colink_proto::*;
use futures_lite::stream::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions},
    types::FieldTable,
    ConnectionProperties,
};
use redis::{
    streams::{StreamReadOptions, StreamReadReply},
    AsyncCommands, FromRedisValue,
};
use secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tonic::{
    metadata::MetadataValue,
    transport::{Certificate, Channel, ClientTlsConfig, Identity},
    Status,
};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthContent {
    pub privilege: String,
    pub user_id: String,
    pub exp: i64,
}

#[derive(Clone)]
pub struct CoLink {
    pub(crate) core_addr: String,
    pub(crate) jwt: String,
    pub(crate) task_id: String,
    pub(crate) ca_certificate: Option<Certificate>,
    pub(crate) identity: Option<Identity>,
    #[cfg(feature = "variable_transfer")]
    pub(crate) vt_p2p_ctx: Arc<crate::extensions::variable_transfer::p2p_inbox::VtP2pCtx>,
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl CoLink {
    pub fn new(core_addr: &str, jwt: &str) -> Self {
        Self {
            core_addr: core_addr.to_string(),
            jwt: jwt.to_string(),
            task_id: "".to_string(),
            ca_certificate: None,
            identity: None,
            #[cfg(feature = "variable_transfer")]
            vt_p2p_ctx: Arc::new(
                crate::extensions::variable_transfer::p2p_inbox::VtP2pCtx::default(),
            ),
        }
    }

    pub fn ca_certificate(mut self, ca_certificate: &str) -> Self {
        let ca_certificate = std::fs::read(ca_certificate).unwrap();
        let ca_certificate = Certificate::from_pem(ca_certificate);
        self.ca_certificate = Some(ca_certificate);
        self
    }

    pub fn identity(mut self, client_cert: &str, client_key: &str) -> Self {
        let client_cert = std::fs::read(client_cert).unwrap();
        let client_key = std::fs::read(client_key).unwrap();
        let identity = Identity::from_pem(client_cert, client_key);
        self.identity = Some(identity);
        self
    }

    async fn _grpc_connect(&self, address: &str) -> Result<CoLinkClient<Channel>, Error> {
        let channel = if self.ca_certificate.is_none() && self.identity.is_none() {
            Channel::builder(address.parse()?).connect().await?
        } else {
            let mut tls = ClientTlsConfig::new();
            if self.ca_certificate.is_some() {
                tls = tls.ca_certificate(self.ca_certificate.clone().unwrap());
            }
            if self.identity.is_some() {
                tls = tls.identity(self.identity.clone().unwrap());
            }
            Channel::builder(address.parse()?)
                .tls_config(tls)?
                .connect()
                .await?
        };
        let client = CoLinkClient::new(channel);
        Ok(client)
    }

    pub fn set_task_id(&mut self, task_id: &str) {
        self.task_id = task_id.to_string();
        #[cfg(feature = "variable_transfer")]
        {
            self.vt_p2p_ctx =
                Arc::new(crate::extensions::variable_transfer::p2p_inbox::VtP2pCtx::default());
        }
    }

    pub fn get_task_id(&self) -> Result<String, String> {
        if self.task_id.is_empty() {
            return Err("task_id not found".to_string());
        }
        Ok(self.task_id.clone())
    }

    pub fn get_user_id(&self) -> Result<String, String> {
        let auth_content = decode_jwt_without_validation(&self.jwt)?;
        Ok(auth_content.user_id)
    }

    pub fn get_core_addr(&self) -> Result<String, String> {
        if self.core_addr.is_empty() {
            return Err("core_addr not found".to_string());
        }
        Ok(self.core_addr.clone())
    }

    pub fn update_jwt(&mut self, new_jwt: &str) -> Result<(), String> {
        self.jwt = new_jwt.to_string();
        Ok(())
    }

    pub async fn import_user(
        &self,
        public_key: &secp256k1::PublicKey,
        signature_timestamp: i64,
        expiration_timestamp: i64,
        signature: &[u8],
    ) -> Result<String, Error> {
        let public_key_vec = public_key.serialize().to_vec();
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let response = client
            .import_user(generate_request(
                &self.jwt,
                UserConsent {
                    public_key: public_key_vec,
                    signature_timestamp,
                    expiration_timestamp,
                    signature: signature.to_vec(),
                },
            ))
            .await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().jwt.clone())
    }

    /// The default expiration time is 1 day later. If you want to specify an expiration time, use refresh_token_with_expiration_time instead.
    pub async fn generate_token(&self, privilege: &str) -> Result<String, Error> {
        self.generate_token_with_expiration_time(chrono::Utc::now().timestamp() + 86400, privilege)
            .await
    }

    pub async fn generate_token_with_expiration_time(
        &self,
        expiration_time: i64,
        privilege: &str,
    ) -> Result<String, Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let response = client
            .generate_token(generate_request(
                &self.jwt,
                GenerateTokenRequest {
                    expiration_time,
                    privilege: privilege.to_string(),
                    ..Default::default()
                },
            ))
            .await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().jwt.clone())
    }

    pub async fn generate_token_with_signature(
        &self,
        public_key: &secp256k1::PublicKey,
        signature_timestamp: i64,
        expiration_timestamp: i64,
        signature: &[u8],
    ) -> Result<String, Error> {
        let public_key_vec = public_key.serialize().to_vec();
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let response = client
            .generate_token(generate_request(
                &self.jwt,
                GenerateTokenRequest {
                    expiration_time: expiration_timestamp,
                    privilege: "user".to_string(),
                    user_consent: Some(UserConsent {
                        public_key: public_key_vec,
                        signature_timestamp,
                        expiration_timestamp,
                        signature: signature.to_vec(),
                    }),
                },
            ))
            .await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().jwt.clone())
    }

    pub async fn create_entry(&self, key_name: &str, payload: &[u8]) -> Result<String, Error> {
        if key_name.contains('$') {
            #[cfg(feature = "storage_macro")]
            return self._sm_create_entry(key_name, payload).await;
            #[cfg(not(feature = "storage_macro"))]
            return Err(format!(
                "Storage Macro feature not enabled, but found $ symbol in key name: {}",
                key_name
            )
            .into());
        }

        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            StorageEntry {
                key_name: key_name.to_string(),
                payload: payload.to_vec(),
                ..Default::default()
            },
        );
        let response = client.create_entry(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().key_path.clone())
    }

    pub async fn read_entries(&self, entries: &[StorageEntry]) -> Result<Vec<StorageEntry>, Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            StorageEntries {
                entries: entries.to_vec(),
            },
        );
        let response = client.read_entries(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().entries.clone())
    }

    pub async fn read_entry(&self, key: &str) -> Result<Vec<u8>, Error> {
        if key.contains('$') {
            #[cfg(feature = "storage_macro")]
            return self._sm_read_entry(key).await;
            #[cfg(not(feature = "storage_macro"))]
            return Err(format!(
                "Storage Macro feature not enabled, but found $ symbol in key name: {}",
                key
            )
            .into());
        }

        let storage_entry = if key.contains("::") {
            StorageEntry {
                key_path: key.to_string(),
                ..Default::default()
            }
        } else {
            StorageEntry {
                key_name: key.to_string(),
                ..Default::default()
            }
        };
        let res = self.read_entries(&[storage_entry]).await?;
        Ok(res[0].payload.clone())
    }

    pub async fn update_entry(&self, key_name: &str, payload: &[u8]) -> Result<String, Error> {
        if key_name.contains('$') {
            #[cfg(feature = "storage_macro")]
            return self._sm_update_entry(key_name, payload).await;
            #[cfg(not(feature = "storage_macro"))]
            return Err(format!(
                "Storage Macro feature not enabled, but found $ symbol in key name: {}",
                key_name
            )
            .into());
        }

        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            StorageEntry {
                key_name: key_name.to_string(),
                payload: payload.to_vec(),
                ..Default::default()
            },
        );
        let response = client.update_entry(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().key_path.clone())
    }

    pub async fn delete_entry(&self, key_name: &str) -> Result<String, Error> {
        if key_name.contains('$') {
            #[cfg(feature = "storage_macro")]
            return self._sm_delete_entry(key_name).await;
            #[cfg(not(feature = "storage_macro"))]
            return Err(format!(
                "Storage Macro feature not enabled, but found $ symbol in key name: {}",
                key_name
            )
            .into());
        }

        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            StorageEntry {
                key_name: key_name.to_string(),
                ..Default::default()
            },
        );
        let response = client.delete_entry(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().key_path.clone())
    }

    pub async fn read_keys(
        &self,
        prefix: &str,
        include_history: bool,
    ) -> Result<Vec<StorageEntry>, Error> {
        if prefix.contains('$') {
            #[cfg(feature = "storage_macro")]
            return self._sm_read_keys(prefix, include_history).await;
            #[cfg(not(feature = "storage_macro"))]
            return Err(format!(
                "Storage Macro feature not enabled, but found $ symbol in key name: {}",
                key_name
            )
            .into());
        }

        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            ReadKeysRequest {
                prefix: prefix.to_string(),
                include_history,
            },
        );
        let response = client.read_keys(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().entries.clone())
    }

    pub async fn import_guest_jwt(&self, jwt: &str) -> Result<(), Error> {
        let jwt_decoded = decode_jwt_without_validation(jwt)?;
        self.update_entry(
            &format!("_internal:known_users:{}:guest_jwt", jwt_decoded.user_id),
            jwt.as_bytes(),
        )
        .await?;
        Ok(())
    }

    pub async fn import_core_addr(&self, user_id: &str, core_addr: &str) -> Result<(), Error> {
        self.update_entry(
            &format!("_internal:known_users:{}:core_addr", user_id),
            core_addr.as_bytes(),
        )
        .await?;
        Ok(())
    }

    pub async fn import_forwarding_user_id(
        &self,
        user_id: &str,
        forwarding_user_id: &str,
    ) -> Result<(), Error> {
        self.update_entry(
            &format!("_internal:known_users:{}:forwarding_user_id", user_id),
            forwarding_user_id.as_bytes(),
        )
        .await?;
        Ok(())
    }

    /// The default expiration time is 1 day later. If you want to specify an expiration time, use run_task_with_expiration_time instead.
    pub async fn run_task(
        &self,
        protocol_name: &str,
        protocol_param: &[u8],
        participants: &[Participant],
        require_agreement: bool,
    ) -> Result<String, Error> {
        self.run_task_with_expiration_time(
            protocol_name,
            protocol_param,
            participants,
            require_agreement,
            chrono::Utc::now().timestamp() + 86400,
        )
        .await
    }

    pub async fn run_task_with_expiration_time(
        &self,
        protocol_name: &str,
        protocol_param: &[u8],
        participants: &[Participant],
        require_agreement: bool,
        expiration_time: i64,
    ) -> Result<String, Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            Task {
                protocol_name: protocol_name.to_string(),
                protocol_param: protocol_param.to_vec(),
                participants: participants.to_vec(),
                parent_task: self.task_id.clone(),
                expiration_time,
                require_agreement,
                ..Default::default()
            },
        );
        let response = client.create_task(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().task_id.clone())
    }

    pub async fn confirm_task(
        &self,
        task_id: &str,
        is_approved: bool,
        is_rejected: bool,
        reason: &str,
    ) -> Result<(), Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            ConfirmTaskRequest {
                task_id: task_id.to_string(),
                decision: Some(Decision {
                    is_approved,
                    is_rejected,
                    reason: reason.to_string(),
                    ..Default::default()
                }),
            },
        );
        let response = client.confirm_task(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(())
    }

    pub async fn finish_task(&self, task_id: &str) -> Result<(), Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            Task {
                task_id: task_id.to_string(),
                ..Default::default()
            },
        );
        let response = client.finish_task(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(())
    }

    pub async fn request_info(&self) -> Result<CoLinkInfo, Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(&self.jwt, Empty::default());
        let response = client.request_info(request).await?;
        debug!("RESPONSE={:?}", response);
        let mq_uri = response.get_ref().mq_uri.clone();
        let requestor_ip = response.get_ref().requestor_ip.clone();
        let version = response.get_ref().version.clone();
        let core_public_key_vec: Vec<u8> = response.get_ref().core_public_key.clone();
        let core_public_key: secp256k1::PublicKey =
            match secp256k1::PublicKey::from_slice(&core_public_key_vec) {
                Ok(pk) => pk,
                Err(e) => {
                    return Err(Box::new(Status::invalid_argument(format!(
                        "The public key could not be decoded in compressed serialized format: {:?}",
                        e
                    ))))
                }
            };
        Ok(CoLinkInfo {
            mq_uri,
            core_public_key,
            requestor_ip,
            version,
        })
    }

    pub async fn subscribe(
        &self,
        key_name: &str,
        start_timestamp: Option<i64>,
    ) -> Result<String, Error> {
        let start_timestamp = match start_timestamp {
            Some(start_timestamp) => start_timestamp,
            None => chrono::Utc::now().timestamp_nanos(),
        };
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            SubscribeRequest {
                key_name: key_name.to_string(),
                start_timestamp,
            },
        );
        let response = client.subscribe(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().queue_name.clone())
    }

    pub async fn unsubscribe(&self, queue_name: &str) -> Result<(), Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            MqQueueName {
                queue_name: queue_name.to_string(),
            },
        );
        let response = client.unsubscribe(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(())
    }

    pub async fn new_subscriber(&self, queue_name: &str) -> Result<CoLinkSubscriber, Error> {
        let mq_uri = self.request_info().await?.mq_uri;
        let subscriber = CoLinkSubscriber::new(&mq_uri, queue_name).await?;
        Ok(subscriber)
    }

    pub async fn start_protocol_operator(
        &self,
        protocol_name: &str,
        user_id: &str,
        upgrade: bool,
    ) -> Result<String, Error> {
        self.start_protocol_operator_full_config(
            protocol_name,
            user_id,
            upgrade,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_protocol_operator_full_config(
        &self,
        protocol_name: &str,
        user_id: &str,
        upgrade: bool,
        source_type: &str,
        deploy_mode: &str,
        source: &str,
        vt_public_addr: &str,
    ) -> Result<String, Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            StartProtocolOperatorRequest {
                protocol_name: protocol_name.to_string(),
                user_id: user_id.to_string(),
                upgrade,
                source_type: source_type.to_string(),
                deploy_mode: deploy_mode.to_string(),
                source: source.to_string(),
                vt_public_addr: vt_public_addr.to_string(),
            },
        );
        let response = client.start_protocol_operator(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(response.get_ref().instance_id.clone())
    }

    pub async fn stop_protocol_operator(&self, instance_id: &str) -> Result<(), Error> {
        let mut client = self._grpc_connect(&self.core_addr).await?;
        let request = generate_request(
            &self.jwt,
            ProtocolOperatorInstanceId {
                instance_id: instance_id.to_string(),
            },
        );
        let response = client.stop_protocol_operator(request).await?;
        debug!("RESPONSE={:?}", response);
        Ok(())
    }
}

pub struct CoLinkInfo {
    pub mq_uri: String,
    pub core_public_key: secp256k1::PublicKey,
    pub requestor_ip: String,
    pub version: String,
}

pub enum CoLinkMQType {
    RabbitMQ,
    RedisStream,
}
pub struct CoLinkSubscriber {
    mq_type: CoLinkMQType,
    queue_name: String,
    rabbitmq_consumer: Option<lapin::Consumer>,
    redis_connection: Option<redis::aio::Connection>,
}

impl CoLinkSubscriber {
    pub async fn new(mq_uri: &str, queue_name: &str) -> Result<Self, Error> {
        let uri_parsed = url::Url::parse(mq_uri)?;
        if uri_parsed.scheme().starts_with("redis") {
            let client = redis::Client::open(mq_uri)?;
            let con = client.get_async_connection().await?;
            Ok(Self {
                mq_type: CoLinkMQType::RedisStream,
                queue_name: queue_name.to_string(),
                rabbitmq_consumer: None,
                redis_connection: Some(con),
            })
        } else {
            let mq = lapin::Connection::connect(mq_uri, ConnectionProperties::default()).await?;
            let channel = mq.create_channel().await?;
            let consumer = channel
                .basic_consume(
                    queue_name,
                    "",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await?;
            Ok(Self {
                mq_type: CoLinkMQType::RabbitMQ,
                queue_name: queue_name.to_string(),
                rabbitmq_consumer: Some(consumer),
                redis_connection: None,
            })
        }
    }

    pub async fn get_next(&mut self) -> Result<Vec<u8>, Error> {
        match self.mq_type {
            CoLinkMQType::RabbitMQ => {
                let delivery = self
                    .rabbitmq_consumer
                    .as_mut()
                    .unwrap()
                    .next()
                    .await
                    .expect("error in consumer");
                let delivery = delivery.expect("error in consumer");
                delivery.ack(BasicAckOptions::default()).await?;
                Ok(delivery.data)
            }
            CoLinkMQType::RedisStream => {
                let opts = StreamReadOptions::default()
                    .group(&self.queue_name, uuid::Uuid::new_v4().to_string())
                    .block(0)
                    .count(1);
                let res: StreamReadReply = self
                    .redis_connection
                    .as_mut()
                    .unwrap()
                    .xread_options(&[&self.queue_name], &[">"], &opts)
                    .await?;
                let id = &res.keys[0].ids[0].id;
                let data: Vec<u8> = FromRedisValue::from_redis_value(
                    res.keys[0].ids[0].map.get("payload").unwrap(),
                )?;
                self.redis_connection
                    .as_mut()
                    .unwrap()
                    .xack(&self.queue_name, &self.queue_name, &[id])
                    .await?;
                self.redis_connection
                    .as_mut()
                    .unwrap()
                    .xdel(&self.queue_name, &[id])
                    .await?;
                Ok(data)
            }
        }
    }
}

pub fn generate_request<T>(jwt: &str, data: T) -> tonic::Request<T> {
    let mut request = tonic::Request::new(data);
    let user_token = MetadataValue::try_from(jwt).unwrap();
    request.metadata_mut().insert("authorization", user_token);
    request
}

pub fn decode_jwt_without_validation(jwt: &str) -> Result<AuthContent, String> {
    let split: Vec<&str> = jwt.split('.').collect();
    let payload = match base64::decode_config(split[1], base64::URL_SAFE_NO_PAD) {
        Ok(payload) => payload,
        Err(e) => return Err(format!("Decode Error: {}", e)),
    };
    let auth_content: AuthContent = match serde_json::from_slice(&payload) {
        Ok(auth_content) => auth_content,
        Err(e) => return Err(format!("Decode Error: {}", e)),
    };
    Ok(auth_content)
}

pub fn generate_user() -> (secp256k1::PublicKey, secp256k1::SecretKey) {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut secp256k1::rand::thread_rng());
    (public_key, secret_key)
}

pub fn prepare_import_user_signature(
    user_pub_key: &secp256k1::PublicKey,
    user_sec_key: &secp256k1::SecretKey,
    core_pub_key: &secp256k1::PublicKey,
    expiration_timestamp: i64,
) -> (i64, Vec<u8>) {
    let secp = Secp256k1::new();
    let signature_timestamp = chrono::Utc::now().timestamp();
    let mut msg = user_pub_key.serialize().to_vec();
    msg.extend_from_slice(&signature_timestamp.to_le_bytes());
    msg.extend_from_slice(&expiration_timestamp.to_le_bytes());
    msg.extend_from_slice(&core_pub_key.serialize());
    let mut hasher = Sha256::new();
    hasher.update(&msg);
    let sha256 = hasher.finalize();
    let signature = secp.sign_ecdsa(
        &secp256k1::Message::from_slice(&sha256).unwrap(),
        user_sec_key,
    );
    (signature_timestamp, signature.serialize_compact().to_vec())
}
