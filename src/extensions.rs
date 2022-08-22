#[cfg(feature = "extensions")]
mod get_participant_id;
#[cfg(feature = "extensions")]
mod lock;
#[cfg(feature = "extensions")]
mod read_or_wait;
#[cfg(feature = "registry")]
pub mod registry;
#[cfg(feature = "remote_storage")]
mod remote_storage;
#[cfg(feature = "variable_transfer")]
mod variable_transfer;
