#![allow(dead_code)]

use std::{
    io::Write,
    process::{Command, Output},
};

use tokio::time::{sleep, Duration};

use crate::{error::AuError, rewards::RewardInfo};

#[derive(Debug)]
pub enum ProcessStatus {
    Ok,
    Stoped,
    NeedReset,
}

#[derive(Debug)]
pub enum JoinStatus {
    Yes,
    NotReady,
    NotRunning,
}

#[derive(Debug)]
pub struct TopioCommands {
    operator_user: String,
    exec_dir: String,
}

impl TopioCommands {
    pub fn new(user: &str, exec_dir: &str) -> Self {
        TopioCommands {
            operator_user: String::from(user),
            exec_dir: String::from(exec_dir),
        }
    }

    /// @root
    pub fn kill_topio(&self) -> Result<Output, AuError> {
        let cmd_str = String::from(
            r#"if ps -ef | grep topio | grep -v grep | grep -v upgrader > /dev/null;\
             then ps -ef | grep topio | grep -v grep | grep -v upgrader | awk '{print $2}' | xargs kill -9 ; fi"#,
        );
        let c = Command::new("sh").arg("-c").arg(cmd_str).spawn()?;

        let r = c.wait_with_output()?;

        Ok(r)
    }

    pub fn wget_new_topio(&self, file_link: &str, tar_name: &str) -> Result<Output, AuError> {
        // // tag version:
        // let cmd_str = format!(
        //     r#"cd {} && wget https://github.com/telosprotocol/TOP-chain/releases/download/v{}/topio-{}-release.tar.gz -O topio-{}-release.tar.gz > /dev/null 2>&1 && tar zxvf topio-{}-release.tar.gz > /dev/null 2>&1 "#,
        //     &self.exec_dir, &tag, &tag, &tag, &tag
        // );
        let cmd_str = format!(
            r#"cd {} && wget {} -O {} > /dev/null 2>&1 && tar zxvf {} > /dev/null 2>&1"#,
            &self.exec_dir, file_link, tar_name, tar_name
        );
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .spawn()?;
        let r = c.wait_with_output()?;
        Ok(r)
    }

    /// @root
    /// install specifical version of topio && restart topio safebox.
    pub fn install_new_topio(&self, tag: String) -> Result<Output, AuError> {
        // @root
        let install_cmd_str = format!(
            r#"cd {} && cd topio-{}-release && sudo bash install.sh > /dev/null 2>&1 "#,
            &self.exec_dir, &tag
        );
        let c = Command::new("sudo")
            .args(&["-u", "root"])
            .args(&["sh", "-c"])
            .arg(install_cmd_str)
            .spawn()?;
        _ = c.wait_with_output()?;

        let rest_cmd_str = format!(
            r#"cd {} && cd topio-{}-release && . /etc/profile && bash set_topio.sh > /dev/null 2>&1 "#,
            &self.exec_dir, &tag
        );

        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(rest_cmd_str)
            .spawn()?;
        _ = c.wait_with_output()?;

        // for now install topio will launcher topio-safebox service, which is root' user, we need to kill && restart as user' user
        _ = self.kill_topio()?;
        self.start_safebox()
    }

    pub fn get_version(&self) -> Result<String, AuError> {
        let cmd_str = format!(
            r#"cd {} && topio -v | grep "topio version" "#,
            &self.exec_dir
        );
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        let output = c.wait_with_output()?;
        Ok(std::str::from_utf8(&output.stdout)?
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| !c.is_ascii_control())
            .collect::<String>())
    }

    pub fn start_safebox(&self) -> Result<Output, AuError> {
        let cmd_str = format!(
            r#"cd {} && topio node safebox > /dev/null "#,
            &self.exec_dir
        );
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .spawn()?;
        let r = c.wait_with_output()?;

        Ok(r)
    }

    pub async fn start_join_and_stop(
        &self,
        mining_pub_key: &str,
        pswd: &str,
    ) -> Result<(), AuError> {
        _ = self.set_miner_key(mining_pub_key, pswd)?;
        _ = self.start_topio()?;

        let mut wait_cnt = 0;
        loop {
            sleep(Duration::from_secs(5)).await;
            if wait_cnt >= 120 {
                return Err(AuError::CustomError("node join failed".into()));
            }
            match self.check_is_joined()? {
                JoinStatus::NotReady => {
                    wait_cnt = wait_cnt + 1;
                }
                JoinStatus::Yes => break,
                JoinStatus::NotRunning => {
                    return Err(AuError::CustomError("node not even running".into()));
                }
            };
        }
        _ = self.stop_topio()?;

        Ok(())
    }

    pub fn set_miner_key(&self, mining_pub_key: &str, pswd: &str) -> Result<Output, AuError> {
        let cmd_str = format!(
            r#"cd {} && topio mining setMinerKey {}"#,
            &self.exec_dir, mining_pub_key
        );
        let mut command = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let mut stdin = command.stdin.take().expect("Failed to use stdin");

        let pswd: String = pswd.into();
        std::thread::spawn(move || {
            stdin
                .write_all(pswd.as_bytes())
                .expect("Failed to write to stdin");
        });
        let output = command.wait_with_output()?;

        Ok(output)
    }

    pub fn set_default_account(&self, address: &str, pswd: &str) -> Result<Output, AuError> {
        let cmd_str = format!(
            r#"cd {} && topio wallet setDefaultAccount {}"#,
            &self.exec_dir, address
        );
        let mut command = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let mut stdin = command.stdin.take().expect("Failed to use stdin");

        let pswd: String = pswd.into();
        std::thread::spawn(move || {
            stdin
                .write_all(pswd.as_bytes())
                .expect("Failed to write to stdin");
        });
        let output = command.wait_with_output()?;

        Ok(output)
    }

    pub fn start_topio(&self) -> Result<Output, AuError> {
        let cmd_str = format!(r#"cd {} && topio node startNode"#, &self.exec_dir);
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let r = c.wait_with_output()?;

        Ok(r)
    }

    pub fn stop_topio(&self) -> Result<Output, AuError> {
        let cmd_str = format!(r#"cd {} && topio node stopNode"#, &self.exec_dir);
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let r = c.wait_with_output()?;

        Ok(r)
    }

    pub fn check_is_joined(&self) -> Result<JoinStatus, AuError> {
        let cmd_str = format!(r#"cd {} && topio node isJoined"#, &self.exec_dir);
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        let r = c.wait_with_output()?;
        let output_str = std::str::from_utf8(&r.stdout)?
            .chars()
            .take_while(|c| !c.is_ascii_control())
            .collect::<String>();
        if output_str.contains("YES") {
            Ok(JoinStatus::Yes)
        } else if output_str.contains("not ready") {
            Ok(JoinStatus::NotReady)
        } else if output_str.contains("not running") {
            Ok(JoinStatus::NotRunning)
        } else {
            Err(AuError::CustomError(format!(
                "topio status error: {}",
                &output_str
            )))
        }
    }

    // reward
    pub fn query_reward(&self, address: &str) -> Result<RewardInfo, AuError> {
        let cmd_str = format!(
            r#"cd {} && topio mining queryMinerReward {} "#,
            &self.exec_dir, address
        );
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let output = c.wait_with_output()?;
        let json = json::parse(std::str::from_utf8(&output.stdout)?)?;
        if let Some(reward) = RewardInfo::new_from_json_value(json) {
            Ok(reward)
        } else {
            Err(AuError::CustomError("reward data parse error".into()))
        }
    }

    pub fn claim_reward(&self, address: &str, pswd: &str) -> Result<Output, AuError> {
        _ = self.set_default_account(address, pswd)?;
        let cmd_str = format!(r#"cd {} && topio mining claimMinerReward"#, &self.exec_dir);
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let output = c.wait_with_output()?;
        Ok(output)
    }

    fn get_balance(&self, address: &str, pswd: &str) -> Result<u64, AuError> {
        _ = self.set_default_account(address, pswd)?;
        let cmd_str = String::from(
            r#"topio wallet listAccounts | head -n 5 | grep 'balance' | awk -F ' ' '{print $2}' "#,
        );
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let output = c.wait_with_output()?;
        let vstr = std::str::from_utf8(&output.stdout)?;
        let v = vstr
            .parse::<f64>()
            .map_err(|_| AuError::CustomError(format!("balance parse str f64 error {}", vstr)))?
            as u64;
        Ok(v)
    }

    pub fn transfer_rest_balance(
        &self,
        from_address: &str,
        pswd: &str,
        to_address: &str,
    ) -> Result<Output, AuError> {
        let balance = self.get_balance(from_address, pswd)?;
        let cmd_str = format!(
            r#"cd {} && topio transfer {} {}"#,
            &self.exec_dir, to_address, balance
        );
        let c = Command::new("sudo")
            .args(&["-u", &self.operator_user])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let output = c.wait_with_output()?;
        Ok(output)
    }

    /// @root
    fn check_topio_running(&self) -> Result<Output, AuError> {
        let cmd_str = format!(
            r#"cd {} && ps -ef | grep topio | grep -v grep | grep -i startnode | wc -l"#,
            &self.exec_dir
        );
        let c = Command::new("sudo")
            .args(&["-u", "root"])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        let r = c.wait_with_output()?;
        Ok(r)
    }

    /// @root
    pub fn topio_status(&self) -> Result<ProcessStatus, AuError> {
        let output = self.check_topio_running()?;
        match std::str::from_utf8(&output.stdout)?
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<usize>()?
        {
            0 => Ok(ProcessStatus::Stoped),
            1 => Ok(ProcessStatus::Ok),
            _ => Ok(ProcessStatus::NeedReset),
        }
    }

    /// @root
    fn check_safebox_running(&self) -> Result<Output, AuError> {
        let cmd_str = format!(
            r#"cd {} && ps -ef | grep topio | grep -v grep | grep -i safebox | wc -l "#,
            &self.exec_dir
        );
        let c = Command::new("sudo")
            .args(&["-u", "root"])
            .args(&["sh", "-c"])
            .arg(cmd_str)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        let r = c.wait_with_output()?;
        Ok(r)
    }

    /// @root
    pub fn safebox_status(&self) -> Result<ProcessStatus, AuError> {
        let output = self.check_safebox_running()?;
        match std::str::from_utf8(&output.stdout)?
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<usize>()?
        {
            0 => Ok(ProcessStatus::Stoped),
            1 => Ok(ProcessStatus::Ok),
            _ => Ok(ProcessStatus::NeedReset),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::commands::TopioCommands;

    #[test]
    #[ignore]
    fn test_topio_cmd() {
        let c = TopioCommands::new("top", "/tmp/test_topio_au");

        // let r = c.topio_status();
        // println!("topio_status:{:?}", r);

        // let r = c.safebox_status();
        // println!("safebox_status:{:?}", r);

        // let r = c.set_miner_key(String::from("BKQLB1qlWXqmfltrMuP0u2h8hfq+Wk8JnbzQbP5EG0xqgWUw97wDF7VnsQOlQ0WVvd/Kv1a6ijFKkf8SPwDSWa4="),String::from("1234"));
        // println!("set key result:{:?}", r);

        let r = c.kill_topio();
        println!("kill result:{:?}", r);

        // let r = c.wget_new_topio(&String::from("https://github.com/telosprotocol/TOP-chain/releases/download/v1.8.0/topio-1.8.0-release.tar.gz"),&String::from("topio-1.8.0-release.tar.gz"));
        // println!("wget result:{:?}", r);

        // let r = c.install_new_topio(String::from("1.7.1"));
        // println!("install result:{:?}", r);

        // let r = c.kill_topio();
        // println!("kill result:{:?}", r);

        // let r = c.start_safebox();
        // println!("start_safebox result:{:?}", r);

        // let r = c.get_version();
        // println!("get version result:{:?}", r);

        // let r = c.set_miner_key("BKQLB1qlWXqmfltrMuP0u2h8hfq+Wk8JnbzQbP5EG0xqgWUw97wDF7VnsQOlQ0WVvd/Kv1a6ijFKkf8SPwDSWa4=",String::from("1234"));
        // println!("set key result:{:?}", r);

        // let r = c.start_topio();
        // println!("start result:{:?}", r);

        // let r = c.check_is_joined();
        // println!("check start result:{:?}", r);
    }
}
