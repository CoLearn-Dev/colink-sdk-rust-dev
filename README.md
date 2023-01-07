# CoLink Rust SDK

CoLink SDK helps both application adnd protocol developers access the functionalities provided by [the CoLink server](https://github.com/CoLearn-Dev/colink-server-dev).

- For *application developers*, CoLink SDK allows them to update storage, manage computation requests, and monitor the CoLink server status.
- For *protocol developers*, CoLink SDK allows them to write CoLink Extensions that extend the functionality of CoLink to support new protocols.

## Usage
Add this to your Cargo.toml:
```toml
[dependencies]
colink = "0.2.7"
```

## Getting Started
You can use this SDK to run protocols, update storage, developing protocol operators. Here is a tutorial for you about how to start a greetings task between two users.
- Set up CoLink server.
Please refer to [colinkctl](https://github.com/CoLearn-Dev/colinkctl), and run the command below. For the following steps, we assume you are using the default settings in colinkctl.
```bash
colinkctl enable_dev_env
```
- Create two new terminals and start protocol operator for two users separately.
```bash
cargo run --example protocol_greetings -- --addr http://localhost:8080 --jwt $(sed -n "1,1p" ~/.colink/user_token.txt)
```
```bash
cargo run --example protocol_greetings -- --addr http://localhost:8080 --jwt $(sed -n "2,2p" ~/.colink/user_token.txt)
```
- Run task
```bash
cargo run --example user_run_task http://localhost:8080 $(sed -n "1,2p" ~/.colink/user_token.txt)
```
- Check the output in protocol operators' terminals

## More examples
### Application
```
cargo run --example host_import_user <address> <host_jwt> <expiration_timestamp> # <expiration_timestamp> is optional
```
```
cargo run --example host_import_users <address> <host_jwt> <number> <expiration_timestamp> # <expiration_timestamp> is optional
```
```
cargo run --example host_import_users_and_exchange_guest_jwts <address> <host_jwt> <number> <expiration_timestamp> # <expiration_timestamp> is optional
```
```
cargo run --example host_import_users_and_set_registry <address> <host_jwt> <number> <expiration_timestamp> # <expiration_timestamp> is optional
```
```
cargo run --example user_confirm_task <address> <user_jwt> <task_id> <action> # <action>: approve(default)/reject/ignore
```
```
cargo run --example user_import_guest_jwt <address> <user_jwt> <guest_jwt>
```
```
cargo run --example user_generate_token <address> <user_jwt>
```
```
cargo run --example user_run_local_task <address> <user_jwt>
```
```
cargo run --example user_run_task <address> <user_jwt A> <user_jwt B> <message> # <message> is optional
```
```
cargo run --example user_greetings_to_multiple_users <address> <initiator_jwt> <receiver_jwt A> <receiver_jwt B> <receiver_jwt...
```
```
cargo run --example auto_confirm <address> <user_jwt> <protocol_name>
```
```
cargo run --example get_next_greeting_message <address> <user_jwt> <start_timestamp> # <start_timestamp> is optional
```
```
cargo run --example mtls_request_info <address> <ca_certificate> <client_cert> <client_key>
```
```
cargo run --example user_lock <address> <user_jwt>
```
```
cargo run --example user_policy_module <address> <user_jwt>
```
```
cargo run --example user_remote_storage <address> <user_jwt A> <user_jwt B> <message> # <message> is optional
```
```
cargo run --example user_start_protocol_operator <address> <user_jwt> <protocol_name>
```
```
cargo run --example user_stop_protocol_operator <address> <user_jwt> <instance_id>
```
```
cargo run --example user_wait_task <address> <user_jwt> <target_user_id>
```
```
cargo run --example storage_macro_chunk <address> <user_jwt> <payload_size>
```

### Protocol
```
cargo run --example protocol_greetings -- --addr <address> --jwt <user_jwt>
```
```
cargo run --example protocol_greetings -- --addr <address> --jwt <user_jwt> --ca <ca_cert> --cert <client_cert> --key <client_key>
```
```
cargo run --example protocol_variable_transfer -- --addr <address> --jwt <user_jwt>
```
