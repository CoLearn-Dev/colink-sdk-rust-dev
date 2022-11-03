use crate::{utils::get_colink_home, CoLink};
use rand::Rng;
use std::{
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
        std::fs::remove_dir_all(&working_dir).unwrap();
    }
}

impl Default for InstantServer {
    fn default() -> Self {
        Self::new()
    }
}

impl InstantServer {
    pub fn new() -> Self {
        let colink_home = get_colink_home().unwrap();
        let program = Path::new(&colink_home).join("colink-server");
        if std::fs::metadata(program.clone()).is_err() {
            Command::new("bash")
                .arg("-c")
                .arg("bash -c \"$(curl -fsSL https://raw.githubusercontent.com/CoLearn-Dev/colinkctl/main/install_colink.sh)\"")
                .status()
                .unwrap();
        }
        let instant_server_id = uuid::Uuid::new_v4().to_string();
        let mut port = rand::thread_rng().gen_range(10000..20000);
        while std::net::TcpStream::connect(&format!("127.0.0.1:{}", port)).is_ok() {
            port = rand::thread_rng().gen_range(10000..20000);
        }
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
            .current_dir(working_dir.clone())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        loop {
            if std::fs::metadata(working_dir.join("host_token.txt")).is_ok()
                && std::net::TcpStream::connect(&format!("127.0.0.1:{}", port)).is_ok()
            {
                break;
            }
            std::thread::sleep(core::time::Duration::from_millis(100));
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
