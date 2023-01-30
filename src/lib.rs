#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::uninlined_format_args)]
mod application;
mod protocol;
mod colink_proto {
    tonic::include_proto!("colink");
}
pub use application::{
    decode_jwt_without_validation, generate_user, prepare_import_user_signature, CoLink,
};
pub use colink_proto::*;
pub use protocol::{
    CoLinkProtocol, ProtocolEntry, _colink_parse_args, _protocol_start, async_trait,
};
pub mod extensions;
pub mod utils;
