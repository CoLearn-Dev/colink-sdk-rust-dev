use crate::StorageEntry;

mod append;
mod chunk;
mod dbc;
mod fs;
mod redis;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub(crate) fn _parse_macro(&self, key_name: &str) -> (String, String, String) {
        let split_key = key_name.split(':').collect::<Vec<&str>>();
        let mut macro_type = String::new();
        for s in split_key.iter().rev() {
            if s.contains('$') {
                macro_type = s.to_string();
                break;
            }
        }
        let macro_type_splitter = format!(":{}", macro_type);
        let split_by_macro = key_name.split(&macro_type_splitter).collect::<Vec<&str>>();
        let mut string_after = split_by_macro[split_by_macro.len() - 1].to_string();
        if string_after.starts_with(':') {
            string_after = string_after[1..].to_string();
        }
        (
            split_by_macro[0..(split_by_macro.len() - 1)].join(&macro_type_splitter),
            macro_type.replace('$', ""),
            string_after,
        )
    }

    pub(crate) async fn _sm_create_entry(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let (string_before, macro_type, string_after) = self._parse_macro(key_name);
        match macro_type.as_str() {
            "chunk" => self._create_entry_chunk(&string_before, payload).await,
            "redis" => {
                self._create_entry_redis(&string_before, &string_after, payload)
                    .await
            }
            "fs" => {
                self._create_entry_fs(&string_before, &string_after, payload)
                    .await
            }
            _ => Err(format!(
                "invalid storage macro, found {} in key name {}",
                macro_type, key_name
            )
            .into()),
        }
    }

    pub(crate) async fn _sm_read_entry(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        let key_name = if key_name.contains("::") {
            &key_name
                [key_name.find(':').unwrap() + 2..key_name.rfind('@').unwrap_or(key_name.len())]
        } else {
            key_name
        };
        let (string_before, macro_type, string_after) = self._parse_macro(key_name);
        match macro_type.as_str() {
            "chunk" => self._read_entry_chunk(&string_before).await,
            "redis" => self._read_entry_redis(&string_before, &string_after).await,
            "dbc" => self._read_entry_dbc(&string_before, &string_after).await,
            "fs" => self._read_entry_fs(&string_before, &string_after).await,
            _ => Err(format!(
                "invalid storage macro, found {} in key name {}",
                macro_type, key_name
            )
            .into()),
        }
    }

    pub(crate) async fn _sm_update_entry(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let (string_before, macro_type, string_after) = self._parse_macro(key_name);
        match macro_type.as_str() {
            "chunk" => self._update_entry_chunk(&string_before, payload).await,
            "redis" => {
                self._update_entry_redis(&string_before, &string_after, payload)
                    .await
            }
            "fs" => {
                self._update_entry_fs(&string_before, &string_after, payload)
                    .await
            }
            "append" => self._update_entry_append(&string_before, payload).await,
            _ => Err(format!(
                "invalid storage macro, found {} in key name {}",
                macro_type, key_name
            )
            .into()),
        }
    }

    pub(crate) async fn _sm_delete_entry(&self, key_name: &str) -> Result<String, Error> {
        let (string_before, macro_type, string_after) = self._parse_macro(key_name);
        match macro_type.as_str() {
            "chunk" => self._delete_entry_chunk(&string_before).await,
            "redis" => {
                self._delete_entry_redis(&string_before, &string_after)
                    .await
            }
            "fs" => self._delete_entry_fs(&string_before, &string_after).await,
            _ => Err(format!(
                "invalid storage macro, found {} in key name {}",
                macro_type, key_name
            )
            .into()),
        }
    }

    pub(crate) async fn _sm_read_keys(
        &self,
        prefix: &str,
        include_history: bool,
    ) -> Result<Vec<StorageEntry>, Error> {
        if include_history {
            return Err("Storage Macro: include_history is not supported.".into());
        }
        if !prefix.starts_with(&format!("{}::", self.get_user_id()?)) {
            return Err("prefix must start with the given user_id".into());
        }
        let key_name_prefix = &prefix[prefix.find(':').unwrap() + 2..];
        let (string_before, macro_type, string_after) = self._parse_macro(key_name_prefix);
        let key_list = match macro_type.as_str() {
            "redis" => self._read_keys_redis(&string_before, &string_after).await?,
            "fs" => self._read_keys_fs(&string_before, &string_after).await?,
            _ => {
                return Err(format!(
                    "invalid storage macro, found {} in prefix {}",
                    macro_type, key_name_prefix
                )
                .into());
            }
        };
        let mut res: Vec<StorageEntry> = Vec::new();
        for key in key_list {
            res.push(StorageEntry {
                key_name: Default::default(),
                key_path: format!("{}:{}@0", prefix, key),
                payload: Default::default(),
            });
        }
        Ok(res)
    }
}
