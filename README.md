# CoLink Rust SDK

CoLink SDK helps both application and protocol developers access the functionalities provided by [the CoLink server](https://github.com/CoLearn-Dev/colink-server-dev).

- For *application developers*, CoLink SDK allows them to update storage, manage computation requests, and monitor the CoLink server status.
- For *protocol developers*, CoLink SDK allows them to write CoLink Extensions that extend the functionality of CoLink to support new protocols.

## Usage
Add this to your Cargo.toml:
```toml
[dependencies]
colink = "0.3.10"
```

Enable more features in your Cargo.toml
```
# if you use storage macro dbc
colink = { version = "0.3.10", features = ["storage_macro_dbc"] }
```

## Getting Started
You can use this SDK to run protocols, update storage, developing protocol operators. Here is a tutorial for you about how to start a greetings task between two users.
- Set up CoLink server.
Please refer to [CoLink Server Setup](https://co-learn.notion.site/CoLink-Server-Setup-aa58e481e36e40cba83a002c1f3bd158)
- Use Rust SDK.
Please refer to [CoLink SDK Examples in Rust](https://co-learn.notion.site/CoLink-SDK-Examples-in-Rust-a9b583ac5d764390aeba7293aa63f39d)

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
