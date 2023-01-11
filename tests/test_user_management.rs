use colink::{
    decode_jwt_without_validation,
    extensions::instant_server::{InstantRegistry, InstantServer},
    generate_user, prepare_import_user_signature, CoLink,
};

#[tokio::test]
async fn test_user_management() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let _ir = InstantRegistry::new();
    let is = InstantServer::new();
    let cl = is.get_colink();
    let core_addr = cl.get_core_addr()?;

    let expiration_timestamp = chrono::Utc::now().timestamp() + 86400 * 31;
    let (pk, sk) = generate_user();
    let core_pub_key = cl.request_info().await?.core_public_key;
    let (signature_timestamp, sig) =
        prepare_import_user_signature(&pk, &sk, &core_pub_key, expiration_timestamp);

    let user_jwt = cl
        .import_user(&pk, signature_timestamp, expiration_timestamp, &sig)
        .await?;
    let user_id = decode_jwt_without_validation(&user_jwt)?.user_id;

    let cl = CoLink::new(&core_addr, &user_jwt);
    let new_expiration_timestamp = chrono::Utc::now().timestamp() + 86400 * 60;
    let guest_jwt = cl
        .generate_token_with_expiration_time(new_expiration_timestamp, "guest")
        .await?;
    let guest_auth_content = decode_jwt_without_validation(&guest_jwt)?;
    assert!(guest_auth_content.user_id == user_id);
    assert!(guest_auth_content.privilege == "guest");
    assert!(guest_auth_content.exp == new_expiration_timestamp);

    let cl = CoLink::new(&core_addr, "");
    let (new_signature_timestamp, new_sig) =
        prepare_import_user_signature(&pk, &sk, &core_pub_key, new_expiration_timestamp);
    let new_user_jwt = cl
        .generate_token_with_signature(
            &pk,
            new_signature_timestamp,
            new_expiration_timestamp,
            &new_sig,
        )
        .await?;
    let user_auth_content = decode_jwt_without_validation(&new_user_jwt)?;
    assert!(user_auth_content.user_id == user_id);
    assert!(user_auth_content.privilege == "user");
    assert!(user_auth_content.exp == new_expiration_timestamp);

    Ok(())
}
