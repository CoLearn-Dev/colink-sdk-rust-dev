fn main() {
    tonic_build::compile_protos("proto/colink.proto")
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
    #[cfg(feature = "remote_storage")]
    prost_build::compile_protos(&["proto/colink_remote_storage.proto"], &["proto/"])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
    #[cfg(feature = "registry")]
    prost_build::compile_protos(&["proto/colink_registry.proto"], &["proto/"])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
    #[cfg(feature = "policy_module")]
    prost_build::compile_protos(&["proto/colink_policy_module.proto"], &["proto/"])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}
