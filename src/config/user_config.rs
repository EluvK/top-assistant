use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct UserConfigJson {
    accounts: Vec<UserKeystoreAddrPswd>,
    mining_pswd_enc: String,
    topio_package_dir: String,
    topio_user: String,
    minimum_claim_value: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserKeystoreAddrPswd {
    pub address: String,
    pub minerpubkey: String,
}

impl UserConfigJson {
    pub(crate) fn set_pswd(&mut self, enc_pswd: String) {
        self.mining_pswd_enc = enc_pswd;
    }

    pub(crate) fn get_enc_pswd(&self) -> &str {
        &self.mining_pswd_enc
    }

    pub fn user(&self) -> &str {
        &self.topio_user
    }

    pub fn exec_dir(&self) -> &str {
        &self.topio_package_dir
    }

    pub fn get_accounts(&self) -> &Vec<UserKeystoreAddrPswd> {
        &self.accounts
    }

    pub fn get_minimum_claim_value(&self) -> u64 {
        self.minimum_claim_value
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_user_config_serde() {
        let config_str = r#"
        {
            "accounts": [
                {
                    "address": "Txxxx",
                    "minerpubkey": "Bkkkkk"
                },
                {
                    "address": "Txxxx",
                    "minerpubkey": "Bkkkkk"
                }
            ],
            "mining_pswd_enc": "03215912372a4f0330affa7167ea1dbbec8253d7ea810b649adb8e35494453b21ba701421dcbc2040bacda2d5b9ea7bd0b",
            "topio_package_dir": "/home/top",
            "topio_user": "top",
            "minimum_claim_value": 2000
        }
        "#;
        let user_config: UserConfigJson = serde_json::from_str(config_str).unwrap();
        println!("user_config struct :{:?}", user_config);
    }
}
