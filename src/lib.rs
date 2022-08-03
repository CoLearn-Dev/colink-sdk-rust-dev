mod sdk_a;
mod sdk_p;
pub mod colink_proto {
    tonic::include_proto!("colink");
}
pub use colink_proto::*;
pub use sdk_a::{
    decode_jwt_without_validation, generate_user, prepare_import_user_signature, CoLink,
};
pub use sdk_p::{CoLinkProtocol, ProtocolEntry, _colink_parse_args, _protocol_start, async_trait};
