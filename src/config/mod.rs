use std::collections::HashMap;

use serde::{Deserialize, Serialize};

mod user_config;
pub use user_config::UserConfigJson;

mod env_config;
use env_config::EnvConfigJson;

mod au_config;
use au_config::AuConfigJson;
pub(crate) use au_config::ReleaseInfoSourceType;

mod temp_config;
use temp_config::TempConfigJson;

use crate::{
    commands::{read_file, write_file},
    error::AuError,
};

use self::user_config::UserKeystoreAddrPubKey;

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigJson {
    pub user_config: HashMap<String, UserConfigJson>,
    pub env_config: EnvConfigJson,
    pub au_config: AuConfigJson,
    pub temp_config: TempConfigJson,

    // Never serialized.
    #[serde(skip)]
    config_path: String,
}

impl ConfigJson {
    /// Create ConfigJson object with config file path.
    pub fn read_from_file(file_path_str: &str) -> Result<Self, AuError> {
        let content = read_file(file_path_str)?;
        let mut config: Self = serde_json::from_str(&content)?;
        config.config_path = String::from(file_path_str); // save for furture use.
        Ok(config)
    }

    /// Check config file. Try decrypt keystore && encrypt password with machine-id's RSA key.
    ///
    /// Called with `--check` parameter at install.sh
    pub fn check_config_file(file_path_str: &str) -> Result<(), AuError> {
        let content = read_file(file_path_str)?;
        let mut config: Self = serde_json::from_str(&content)?;
        config.config_path = String::from(file_path_str); // save for furture use.

        config.try_encrypt_password();
        // config.try_decrypt_keystore()?;
        config.update_config_file()?;
        Ok(())
    }

    /// Write config back to config.json file.
    ///
    /// Called after alter config's content.
    pub fn update_config_file(&self) -> Result<(), AuError> {
        write_file(&self.config_path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }

    fn try_encrypt_password(&mut self) {
        for (id, user_config) in self.user_config.iter_mut() {
            let pswd = self
                .temp_config
                .take_pswd(id)
                .expect(format!("error get pswd of {}", id).as_str());
            user_config.set_pswd(self.env_config.encrypt(pswd))
        }
    }

    /// Decode encrypted password with machine-id's RSA key
    pub fn fetch_password(&self, id: &String) -> String {
        self.env_config
            .decrypt(self.user_config.get(id).unwrap().get_enc_pswd())
    }

    pub fn accounts_info(&self, id: &String) -> &Vec<UserKeystoreAddrPubKey> {
        self.user_config.get(id).unwrap().get_accounts()
    }
}
