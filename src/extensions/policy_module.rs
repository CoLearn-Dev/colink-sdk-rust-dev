use crate::colink_proto::*;
pub use colink_policy_module_proto::*;
use prost::Message;
mod colink_policy_module_proto {
    include!(concat!(env!("OUT_DIR"), "/colink_policy_module.rs"));
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub async fn policy_module_start(&self) -> Result<(), Error> {
        let lock = self.lock("_policy_module:settings").await?;
        let (mut settings, timestamp): (Settings, i64) = match self
            .read_entries(&[StorageEntry {
                key_name: "_policy_module:settings".to_string(),
                ..Default::default()
            }])
            .await
        {
            Ok(res) => (
                prost::Message::decode(&*res[0].payload)?,
                get_timestamp(&res[0].key_path),
            ),
            Err(_) => (Default::default(), 0),
        };
        if settings.enable {
            self.unlock(lock).await?;
            return self.wait_for_applying(timestamp).await; // Wait for the current timestamp to be applied.
        }
        settings.enable = true;
        let mut payload = vec![];
        settings.encode(&mut payload).unwrap();
        let timestamp = get_timestamp(
            &self
                .update_entry("_policy_module:settings", &payload)
                .await?,
        );
        self.unlock(lock).await?;
        let participants = vec![Participant {
            user_id: self.get_user_id()?,
            role: "local".to_string(),
        }];
        self.run_task("policy_module", Default::default(), &participants, false)
            .await?;
        self.wait_for_applying(timestamp).await
    }

    pub async fn policy_module_stop(&self) -> Result<(), Error> {
        let lock = self.lock("_policy_module:settings").await?;
        let mut settings: Settings = match self.read_entry("_policy_module:settings").await {
            Ok(res) => prost::Message::decode(&*res)?,
            Err(_) => Default::default(),
        };
        if !settings.enable {
            self.unlock(lock).await?;
            return Ok(()); // Return directly here because we only release the lock after the policy module truly stopped.
        }
        settings.enable = false;
        let mut payload = vec![];
        settings.encode(&mut payload).unwrap();
        let timestamp = get_timestamp(
            &self
                .update_entry("_policy_module:settings", &payload)
                .await?,
        );
        let res = self.wait_for_applying(timestamp).await;
        self.unlock(lock).await?; // Unlock after the policy module truly stopped.
        res
    }

    pub async fn policy_module_get_rules(&self) -> Result<Vec<Rule>, Error> {
        let settings: Settings = match self.read_entry("_policy_module:settings").await {
            Ok(res) => prost::Message::decode(&*res)?,
            Err(_) => Default::default(),
        };
        Ok(settings.rules)
    }

    pub async fn policy_module_add_rule(&self, rule: &Rule) -> Result<String, Error> {
        let lock = self.lock("_policy_module:settings").await?;
        let mut settings: Settings = match self.read_entry("_policy_module:settings").await {
            Ok(res) => prost::Message::decode(&*res)?,
            Err(_) => Default::default(),
        };
        let rule_id = uuid::Uuid::new_v4().to_string();
        let mut rule = rule.clone();
        rule.rule_id = rule_id.clone();
        settings.rules.push(rule);
        let mut payload = vec![];
        settings.encode(&mut payload).unwrap();
        let timestamp = get_timestamp(
            &self
                .update_entry("_policy_module:settings", &payload)
                .await?,
        );
        self.unlock(lock).await?;
        if settings.enable {
            self.wait_for_applying(timestamp).await?;
        }
        Ok(rule_id)
    }

    pub async fn policy_module_remove_rule(&self, rule_id: &str) -> Result<(), Error> {
        let lock = self.lock("_policy_module:settings").await?;
        let mut settings: Settings = match self.read_entry("_policy_module:settings").await {
            Ok(res) => prost::Message::decode(&*res)?,
            Err(_) => Default::default(),
        };
        settings.rules.retain(|x| x.rule_id != rule_id);
        let mut payload = vec![];
        settings.encode(&mut payload).unwrap();
        let timestamp = get_timestamp(
            &self
                .update_entry("_policy_module:settings", &payload)
                .await?,
        );
        self.unlock(lock).await?;
        if settings.enable {
            self.wait_for_applying(timestamp).await?;
        }
        Ok(())
    }

    async fn wait_for_applying(&self, timestamp: i64) -> Result<(), Error> {
        let key = "_policy_module:applied_settings_timestamp";
        let start_timestamp = match self
            .read_entries(&[StorageEntry {
                key_name: key.to_string(),
                ..Default::default()
            }])
            .await
        {
            Ok(res) => {
                let applied_settings_timestamp =
                    i64::from_le_bytes(<[u8; 8]>::try_from(&*res[0].payload).unwrap());
                if applied_settings_timestamp >= timestamp {
                    return Ok(());
                }
                get_timestamp(&res[0].key_path) + 1
            }
            Err(_) => 0,
        };
        let queue_name = self.subscribe(key, Some(start_timestamp)).await?;
        let mut subscriber = self.new_subscriber(&queue_name).await?;
        loop {
            let data = subscriber.get_next().await?;
            let message: SubscriptionMessage = Message::decode(&*data).unwrap();
            if message.change_type != "delete" {
                let applied_settings_timestamp =
                    i64::from_le_bytes(<[u8; 8]>::try_from(&*message.payload).unwrap());
                if applied_settings_timestamp >= timestamp {
                    break;
                }
            }
        }
        self.unsubscribe(&queue_name).await?;
        Ok(())
    }
}

fn get_timestamp(key_path: &str) -> i64 {
    let pos = key_path.rfind('@').unwrap();
    key_path[pos + 1..].parse().unwrap()
}
