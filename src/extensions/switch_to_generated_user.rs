use crate::decode_jwt_without_validation;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    async fn generate_user_and_import(&self) -> Result<String, Error> {
        let auth_content = decode_jwt_without_validation(&self.jwt)?;
        let expiration_timestamp = auth_content.exp;
        let (pk, sk) = crate::generate_user();
        let (_, core_pub_key, _) = self.request_info().await?;
        let (signature_timestamp, sig) =
            crate::prepare_import_user_signature(&pk, &sk, &core_pub_key, expiration_timestamp);
        self.import_user(&pk, signature_timestamp, expiration_timestamp, &sig)
            .await
    }

    pub async fn switch_to_generated_user(&mut self) -> Result<(), Error> {
        self.jwt = self.generate_user_and_import().await?;
        self.wait_user_init().await?;
        Ok(())
    }

    pub async fn clone_and_switch_to_generated_user(&self) -> Result<Self, Error> {
        let cl = Self::new(&self.core_addr, &self.generate_user_and_import().await?);
        cl.wait_user_init().await?;
        Ok(cl)
    }
}
