use async_recursion::async_recursion;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    async fn _sm_fs_get_path(
        &self,
        path_key_name: &str,
        path_suffix: &str,
    ) -> Result<PathBuf, Error> {
        let path_key = format!("{}:path", path_key_name);
        let mut path = String::from_utf8(self.read_entry(&path_key).await?)?;
        if !path_suffix.is_empty() {
            let path_suffix = path_suffix.replace(':', "/");
            path += "/";
            path += &path_suffix;
        }
        let path = PathBuf::from(path);
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        Ok(path)
    }

    #[async_recursion]
    pub(crate) async fn _create_entry_fs(
        &self,
        path_key_name: &str,
        path_suffix: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let path = self._sm_fs_get_path(path_key_name, path_suffix).await?;
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
        path_key_name: &str,
        path_suffix: &str,
    ) -> Result<Vec<u8>, Error> {
        let path = self._sm_fs_get_path(path_key_name, path_suffix).await?;
        let data = tokio::fs::read(path).await?;
        Ok(data)
    }

    #[async_recursion]
    pub(crate) async fn _update_entry_fs(
        &self,
        path_key_name: &str,
        path_suffix: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let path = self._sm_fs_get_path(path_key_name, path_suffix).await?;
        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(payload).await?;
        Ok("ok".to_string())
    }

    #[async_recursion]
    pub(crate) async fn _append_entry_fs(
        &self,
        path_key_name: &str,
        path_suffix: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let path = self._sm_fs_get_path(path_key_name, path_suffix).await?;
        let lock_token = self.lock(&path.to_string_lossy()).await?;
        // use a closure to prevent locking forever caused by errors
        let res = async {
            let mut file = tokio::fs::OpenOptions::new()
                .append(true)
                .open(path)
                .await?;
            file.write_all(payload).await?;
            Ok::<String, Error>("ok".to_string())
        }
        .await;
        self.unlock(lock_token).await?;
        res
    }

    #[async_recursion]
    pub(crate) async fn _delete_entry_fs(
        &self,
        path_key_name: &str,
        path_suffix: &str,
    ) -> Result<String, Error> {
        let path = self._sm_fs_get_path(path_key_name, path_suffix).await?;
        tokio::fs::remove_file(path).await?;
        Ok("ok".to_string())
    }
}
