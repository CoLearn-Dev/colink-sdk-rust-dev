use async_recursion::async_recursion;
use tokio::io::AsyncWriteExt;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    #[async_recursion]
    pub(crate) async fn _create_entry_fs(
        &self,
        _string_before: &str,
        path: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .await?;
        file.write_all(payload).await?;
        Ok("ok".to_string())
    }

    #[async_recursion]
    pub(crate) async fn _read_entry_fs(
        &self,
        _string_before: &str,
        path: &str,
    ) -> Result<Vec<u8>, Error> {
        let data = tokio::fs::read(path).await?;
        Ok(data)
    }

    #[async_recursion]
    pub(crate) async fn _update_entry_fs(
        &self,
        _string_before: &str,
        path: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(payload).await?;
        Ok("ok".to_string())
    }

    #[async_recursion]
    pub(crate) async fn _delete_entry_fs(
        &self,
        _string_before: &str,
        path: &str,
    ) -> Result<String, Error> {
        tokio::fs::remove_file(path).await?;
        Ok("ok".to_string())
    }
}
