# CoLink Rust SDK

For application developers, CoLink SDK provides a toolkit for application developers which allows them to update storage, manage computation requests, and monitor CoLink server status.

For protocol developers, CoLink SDK provides a toolkit for protocol developers which allows them to write CoLink Extensions that extend the functionality of CoLink to support new protocols.

## Application
```
cargo run --example host_import_user <address> <host_jwt> <expiration_timestamp> # <expiration_timestamp> is optional
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
cargo run --example mtls_request_core_info <address> <ca_certificate> <client_cert> <client_key>
```
```
cargo run --example user_lock <address> <user_jwt>
```

## Protocol
```
cargo run --example protocol_greetings -- --addr <address> --jwt <user_jwt>
```
```
cargo run --example protocol_greetings -- --addr <address> --jwt <user_jwt> --ca <ca_cert> --cert <client_cert> --key <client_key>
```
```
cargo run --example protocol_variable_transfer -- --addr <address> --jwt <user_jwt>
```
