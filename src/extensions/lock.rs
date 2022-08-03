use rand::Rng;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    /// The default retry time cap is 100 ms. If you want to specify a retry time cap, use lock_with_retry_time instead.
    pub async fn lock(&self, key: &str) -> Result<CoLinkLockToken, Error> {
        self.lock_with_retry_time(key, 100).await
    }

    pub async fn lock_with_retry_time(
        &self,
        key: &str,
        retry_time_cap_in_ms: u64,
    ) -> Result<CoLinkLockToken, Error> {
        let mut sleep_time_cap = 1;
        let rnd_num = rand::thread_rng().gen::<i32>();
        loop {
            if self
                .create_entry(&format!("_lock:{}", key), &rnd_num.to_le_bytes())
                .await
                .is_ok()
            {
                break;
            }
            let st = rand::thread_rng().gen_range(0..sleep_time_cap);
            tokio::time::sleep(tokio::time::Duration::from_millis(st)).await;
            sleep_time_cap *= 2;
            if sleep_time_cap > retry_time_cap_in_ms {
                sleep_time_cap = retry_time_cap_in_ms;
            }
        }
        Ok(CoLinkLockToken {
            key: key.to_string(),
            rnd_num,
        })
    }

    pub async fn unlock(&self, lock_token: CoLinkLockToken) -> Result<(), Error> {
        let rnd_num_in_storage = self
            .read_entry(&format!("_lock:{}", lock_token.key))
            .await?;
        let rnd_num_in_storage =
            i32::from_le_bytes(<[u8; 4]>::try_from(rnd_num_in_storage).unwrap());
        if rnd_num_in_storage == lock_token.rnd_num {
            self.delete_entry(&format!("_lock:{}", lock_token.key))
                .await?;
        } else {
            Err("Invalid token.")?
        }
        Ok(())
    }
}

#[cfg(feature = "lock")]
pub struct CoLinkLockToken {
    key: String,
    rnd_num: i32,
}
