mod chunk;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    fn _parse_macro(&self, key_name: &str) -> (String, String, String) {
        let split_key = key_name.split(':').collect::<Vec<&str>>();
        let mut macro_type = String::new();
        for s in split_key.iter().rev() {
            if s.contains('$') {
                macro_type = s.replace('$', "");
                break;
            }
        }
        let split_by_macro = key_name
            .split(&format!(":${}:", macro_type))
            .collect::<Vec<&str>>();

        (
            split_by_macro[0].to_string(),
            macro_type,
            split_by_macro[1].to_string(),
        )
    }

    pub(crate) async fn _sm_create_entry(
        &self,
        key_name: &str,
        payload: &[u8],
    ) -> Result<String, Error> {
        let parsed_tuple = self._parse_macro(key_name);
        let macro_type = parsed_tuple.1.as_str();
        let string_before = parsed_tuple.0.as_str();
        match macro_type {
            "chunk" => self._create_entry_chunk(string_before, payload).await,
            _ => Err(format!(
                "invalid storage option, found {} in key name {}",
                macro_type, key_name
            )
            .into()),
        }
    }

    pub(crate) async fn _sm_read_entry(&self, key_name: &str) -> Result<Vec<u8>, Error> {
        let parsed_tuple = self._parse_macro(key_name);
        let macro_type = parsed_tuple.1.as_str();
        let string_before = parsed_tuple.0.as_str();
        match macro_type {
            "chunk" => self._read_entry_chunk(string_before).await,
            _ => Err(format!(
                "invalid storage option, found {} in key name {}",
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
        let parsed_tuple = self._parse_macro(key_name);
        let macro_type = parsed_tuple.1.as_str();
        let string_before = parsed_tuple.0.as_str();
        match macro_type {
            "chunk" => self._update_entry_chunk(string_before, payload).await,
            _ => Err(format!(
                "invalid storage option, found {} in key name {}",
                macro_type, key_name
            )
            .into()),
        }
    }

    pub(crate) async fn _sm_delete_entry(&self, key_name: &str) -> Result<String, Error> {
        let parsed_tuple = self._parse_macro(key_name);
        let macro_type = parsed_tuple.1.as_str();
        let string_before = parsed_tuple.0.as_str();
        match macro_type {
            "chunk" => self._delete_entry_chunk(string_before).await,
            _ => Err(format!(
                "invalid storage option, found {} in key name {}",
                macro_type, key_name
            )
            .into()),
        }
    }
}
