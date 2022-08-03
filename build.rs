fn main() {
    tonic_build::compile_protos("proto/colink.proto")
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
    #[cfg(feature = "variable_transfer")]
    prost_build::compile_protos(&["proto/colink_remote_storage.proto"], &["proto/"])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}
