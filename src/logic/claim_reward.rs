use rand::Rng;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::{
    commands::TopioCommands, config::ConfigJson, error::AuError, frequency::FrequencyControl,
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
        let cmd = TopioCommands::new(
            self.config.user_config.user(),
            self.config.user_config.exec_dir(),
        );
        let pswd = self.config.fetch_password();
        let accounts = self.config.accounts_info();
        for ac in accounts {
            let r = cmd.query_reward(&ac.address)?;
            // utop -> top rate, need * 100_000
            if r.unclaimed_gt(self.config.user_config.get_minimum_claim_value() * 1_000_000) {
                _ = cmd.claim_reward(&ac.address, &pswd)?
            }
        }
        _ = self.do_transfer_balance()?;
        Ok(())
    }

    fn do_transfer_balance(&self) -> Result<(), AuError> {
        let cmd = TopioCommands::new(
            self.config.user_config.user(),
            self.config.user_config.exec_dir(),
        );
        let pswd = self.config.fetch_password();
        let accounts = self.config.accounts_info();
        let target_address = self.config.user_config.get_balance_target_address();
        for ac in accounts {
            if !ac.address.eq_ignore_ascii_case(target_address) {
                _ = cmd.transfer_rest_balance(&ac.address, &pswd, target_address)?;
            }
        }
        Ok(())
    }
}
