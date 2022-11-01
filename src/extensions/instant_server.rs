use crate::{utils::get_colink_home, CoLink};
use rand::Rng;
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    sync::Arc,
};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub(crate) struct InstantServer {
    id: String,
    port: i32,
    process: Child,
}

impl Drop for InstantServer {
    fn drop(&mut self) {
        self.process.kill().unwrap();
        let colink_home = get_colink_home().unwrap();
        let working_dir = Path::new(&colink_home)
            .join("instant_servers")
            .join(self.id.clone());
        std::fs::remove_dir_all(&working_dir).unwrap();
    }
}

impl InstantServer {
    async fn create_user(&self) -> Result<String, Error> {
        let colink_home = get_colink_home().unwrap();
        let working_dir = Path::new(&colink_home)
            .join("instant_servers")
            .join(self.id.clone());
        loop {
            if std::fs::metadata(working_dir.join("host_token.txt")).is_ok()
                && std::net::TcpStream::connect(&format!("127.0.0.1:{}", self.port)).is_ok()
            {
                break;
            }
            tokio::time::sleep(core::time::Duration::from_millis(100)).await;
        }
        let host_token: String =
            String::from_utf8_lossy(&std::fs::read(working_dir.join("host_token.txt"))?).parse()?;
        let cl = CoLink::new(&format!("http://127.0.0.1:{}", self.port), &host_token);
        let expiration_timestamp = chrono::Utc::now().timestamp() + 86400 * 31;
        let (pk, sk) = crate::generate_user();
        let (_, core_pub_key, _) = cl.request_info().await?;
        let (signature_timestamp, sig) =
            crate::prepare_import_user_signature(&pk, &sk, &core_pub_key, expiration_timestamp);
        let user_jwt = cl
            .import_user(&pk, signature_timestamp, expiration_timestamp, &sig)
            .await?;
        Ok(user_jwt)
    }
}

impl crate::application::CoLink {
    fn start_instant_server() -> InstantServer {
        let instant_server_id = uuid::Uuid::new_v4().to_string();
        let port = rand::thread_rng().gen_range(10000..20000);
        let colink_home = get_colink_home().unwrap();
        let program = Path::new(&colink_home).join("colink-server");
        // TODO install colink
        let working_dir = Path::new(&colink_home)
            .join("instant_servers")
            .join(instant_server_id.clone());
        std::fs::create_dir_all(&working_dir).unwrap();
        let child = Command::new(program)
            .args([
                "--address",
                "0.0.0.0",
                "--port",
                &port.to_string(),
                "--mq-amqp",
                "amqp://guest:guest@localhost:5672",
                "--mq-api",
                "http://guest:guest@localhost:15672/api",
                "--mq-prefix",
                &format!("colink-instant-server-{}", port),
                "--core-uri",
                &format!("http://127.0.0.1:{}", port),
                "--inter-core-reverse-mode",
            ])
            .env("COLINK_HOME", colink_home)
            .current_dir(working_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        InstantServer {
            id: instant_server_id,
            port,
            process: child,
        }
    }

    pub async fn new_instant_server() -> Self {
        let instant_server = Self::start_instant_server();
        let user_jwt = instant_server.create_user().await.unwrap();
        let cl = Self {
            core_addr: format!("http://127.0.0.1:{}", instant_server.port),
            jwt: user_jwt,
            task_id: "".to_string(),
            ca_certificate: None,
            identity: None,
            _instant_server_process: Some(Arc::new(instant_server)),
        };
        cl.wait_user_init().await.unwrap();
        cl
    }
}
