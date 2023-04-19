use crate::{utils::get_colink_home, CoLink};
use rand::Rng;
use std::{
    fs::File,
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
};

pub struct InstantServer {
    id: String,
    port: i32,
    host_token: String,
    process: Child,
}

impl Drop for InstantServer {
    fn drop(&mut self) {
        Command::new("pkill")
            .arg("-9")
            .arg("-P")
            .arg(&self.process.id().to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        self.process.kill().unwrap();
        let colink_home = get_colink_home().unwrap();
        let working_dir = Path::new(&colink_home)
            .join("instant_servers")
            .join(self.id.clone());
        std::fs::remove_dir_all(working_dir).unwrap();
    }
}

impl Default for InstantServer {
    fn default() -> Self {
        Self::new()
    }
}

impl InstantServer {
    pub fn new() -> Self {
        InstantServer::new_with_config(
            r#"
            [policy_module]
            operator_num = 1
            [[policy_module.create_entry]]
            key_name = "_policy_module:init:accept_all_tasks"
            value = "true"
            
            [remote_storage]
            operator_num = 1
            
            [registry]
            operator_num = 1
            "#,
        )
    }

    pub fn new_with_config(user_init_config: &str) -> Self {
        let colink_home = get_colink_home().unwrap();
        let program = Path::new(&colink_home).join("colink-server");
        if std::fs::metadata(program.clone()).is_err() {
            Command::new("bash")
                .arg("-c")
                .arg("bash -c \"$(curl -fsSL https://raw.githubusercontent.com/CoLearn-Dev/colinkctl/main/install_colink.sh)\"")
                .env("COLINK_INSTALL_SERVER_ONLY", "true")
                .env("COLINK_INSTALL_SILENT", "true")
                .env("COLINK_SERVER_VERSION", "v0.3.5")
                .status()
                .unwrap();
        }
        let instant_server_id = uuid::Uuid::new_v4().to_string();
        let mut port = rand::thread_rng().gen_range(10000..20000);
        while std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            port = rand::thread_rng().gen_range(10000..20000);
        }
        let working_dir = Path::new(&colink_home)
            .join("instant_servers")
            .join(instant_server_id.clone());
        std::fs::create_dir_all(&working_dir).unwrap();
        let mut user_init_config_file =
            std::fs::File::create(&Path::new(&working_dir).join("user_init_config.toml")).unwrap();
        user_init_config_file
            .write_all(user_init_config.as_bytes())
            .unwrap();
        let mq_uri = if std::env::var("COLINK_SERVER_MQ_URI").is_ok() {
            Some(std::env::var("COLINK_SERVER_MQ_URI").unwrap())
        } else {
            None
        };
        let mq_api = if std::env::var("COLINK_SERVER_MQ_API").is_ok() {
            Some(std::env::var("COLINK_SERVER_MQ_API").unwrap())
        } else {
            None
        };
        let (mq_uri, mq_api) = std::thread::spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    if mq_uri.is_some() {
                        let mq_uri = mq_uri.clone().unwrap();
                        if mq_uri.starts_with("amqp") {
                            lapin::Connection::connect(
                                &mq_uri,
                                lapin::ConnectionProperties::default(),
                            )
                            .await
                            .unwrap();
                            if mq_api.is_some() {
                                let res = reqwest::get(&mq_api.clone().unwrap()).await.unwrap();
                                assert!(res.status() == hyper::StatusCode::OK);
                            }
                        } else if mq_uri.starts_with("redis") {
                            let client = redis::Client::open(mq_uri).unwrap();
                            let _con = client.get_async_connection().await.unwrap();
                        } else {
                            panic!("mq_uri({}) is not supported.", mq_uri);
                        }
                    }
                });
            (mq_uri, mq_api)
        })
        .join()
        .unwrap();
        let mut args = vec![
            "--address".to_string(),
            "0.0.0.0".to_string(),
            "--port".to_string(),
            port.to_string(),
            "--mq-prefix".to_string(),
            format!("colink-instant-server-{}", port),
            "--core-uri".to_string(),
            format!("http://127.0.0.1:{}", port),
            "--inter-core-reverse-mode".to_string(),
        ];
        if let Some(mq_uri) = mq_uri {
            args.push("--mq-uri".to_string());
            args.push(mq_uri);
        }
        if let Some(mq_api) = mq_api {
            args.push("--mq-api".to_string());
            args.push(mq_api);
        }
        let child = Command::new(program)
            .args(&args)
            .env("COLINK_HOME", colink_home)
            .current_dir(working_dir.clone())
            .spawn()
            .unwrap();
        loop {
            if std::fs::metadata(working_dir.join("host_token.txt")).is_ok()
                && std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
            {
                break;
            }
            std::thread::sleep(core::time::Duration::from_millis(10));
        }
        let host_token: String =
            String::from_utf8_lossy(&std::fs::read(working_dir.join("host_token.txt")).unwrap())
                .parse()
                .unwrap();
        Self {
            id: instant_server_id,
            port,
            host_token,
            process: child,
        }
    }

    pub fn get_colink(&self) -> CoLink {
        CoLink::new(&format!("http://127.0.0.1:{}", self.port), &self.host_token)
    }
}

pub struct InstantRegistry {
    _instant_server: InstantServer,
}

impl Drop for InstantRegistry {
    fn drop(&mut self) {
        let colink_home = get_colink_home().unwrap();
        let registry_file = Path::new(&colink_home).join("reg_config");
        std::fs::remove_file(registry_file).unwrap();
    }
}

impl Default for InstantRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InstantRegistry {
    pub fn new() -> Self {
        let is = InstantServer::new();
        let colink_home = get_colink_home().unwrap();
        let registry_file = Path::new(&colink_home).join("reg_config");
        let _file = File::options()
            .write(true)
            .create_new(true)
            .open(registry_file)
            .unwrap();
        let is = std::thread::spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    is.get_colink().switch_to_generated_user().await.unwrap();
                });
            is
        })
        .join()
        .unwrap();
        Self {
            _instant_server: is,
        }
    }
}
