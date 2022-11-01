type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn wait_user_init(&self) -> Result<(), Error> {
        let is_initialized_key = "_internal:_is_initialized";
        loop {
            let res = self.read_entry(is_initialized_key).await;
            if res.is_ok() && res.unwrap()[0] == 1 {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        Ok(())
    }
}
