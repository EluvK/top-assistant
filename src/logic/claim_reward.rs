use rand::{seq::SliceRandom, Rng};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::{
    commands::TopioCommands,
    config::{ConfigJson, UserConfigJson},
    error::AuError,
    frequency::FrequencyControl,
};

pub struct ClaimRewardLogic {
    logic_mutex: Arc<Mutex<i32>>,
    config: Arc<ConfigJson>,
    frequency: Arc<Mutex<FrequencyControl>>,
}

impl ClaimRewardLogic {
    pub async fn loop_run(&self) {
        let mut rng = rand::thread_rng();
        loop {
            {
                if let Ok(_) = self.logic_mutex.try_lock() {
                    let r = self.inner_run();
                    println!("ClaimRewardLogic {:?}", r);
                }
            }
            sleep(Duration::from_secs(rng.gen_range(10..100))).await;
        }
    }
    pub fn new(logic_mutex: Arc<Mutex<i32>>, config: Arc<ConfigJson>) -> Self {
        let interval_base = config.au_config.logic_frequency_base();
        Self {
            logic_mutex,
            config: config,
            frequency: Arc::new(Mutex::new(FrequencyControl::new(
                Duration::from_secs(0),
                Duration::from_secs(10 * 60 * interval_base), // 10 hours
                Duration::from_secs(10 * 60 * interval_base), // 10 hours
                Duration::from_secs(72 * 60 * interval_base), // 72 hours = 3 days
            ))),
        }
    }

    fn inner_run(&self) -> Result<(), AuError> {
        if !self.frequency.lock().unwrap().call_if_allowed() {
            return Ok(());
        }
        _ = self.do_claim_reward()?;
        Ok(())
    }

    fn do_claim_reward(&self) -> Result<(), AuError> {
        let acc_collections: Vec<&String> = self.config.user_config.keys().collect();
        let rand_id = acc_collections.choose(&mut rand::thread_rng()).unwrap();
        let rand_user_config = self.config.user_config.get(*rand_id).unwrap();

        let cmd = TopioCommands::new(rand_user_config.user(), rand_user_config.exec_dir());
        let pswd = self.config.fetch_password(&rand_id);
        let accounts = self.config.accounts_info(&rand_id);
        let mut claim_flag = false;
        for ac in accounts {
            let r = cmd.query_reward(&ac.address)?;
            // utop -> top rate, need * 1_000_000
            if r.unclaimed_gt(rand_user_config.get_minimum_claim_value() * 1_000_000) {
                _ = cmd.claim_reward(&ac.address, &pswd)?;
                claim_flag = true;
            }
        }
        if claim_flag {
            _ = self.do_transfer_balance(rand_id, rand_user_config)?;
        }
        Ok(())
    }

    fn do_transfer_balance(
        &self,
        rand_id: &String,
        rand_user_config: &UserConfigJson,
    ) -> Result<(), AuError> {
        let cmd = TopioCommands::new(rand_user_config.user(), rand_user_config.exec_dir());
        let pswd = self.config.fetch_password(rand_id);
        let accounts = self.config.accounts_info(rand_id);
        let target_address = rand_user_config.get_balance_target_address();
        for ac in accounts {
            if !ac.address.eq_ignore_ascii_case(target_address) {
                let balance = cmd.get_balance(&ac.address, &pswd)?;
                if balance > 100 {
                    _ = cmd.transfer(target_address, balance - 100)?;
                }
            }
        }
        Ok(())
    }
}
