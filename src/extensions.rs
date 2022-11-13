#[cfg(feature = "extensions")]
mod get_participant_id;
#[cfg(feature = "instant_server")]
pub mod instant_server;
#[cfg(feature = "extensions")]
mod lock;
#[cfg(feature = "magic_mount")]
mod magic_mount;
#[cfg(feature = "policy_module")]
pub mod policy_module;
#[cfg(feature = "extensions")]
mod read_or_wait;
#[cfg(feature = "registry")]
pub mod registry;
#[cfg(feature = "remote_storage")]
mod remote_storage;
#[cfg(feature = "extensions")]
mod switch_to_generated_user;
#[cfg(feature = "variable_transfer")]
mod variable_transfer;
#[cfg(feature = "extensions")]
mod wait_task;
#[cfg(feature = "extensions")]
mod wait_user_init;
