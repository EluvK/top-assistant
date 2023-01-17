use json::JsonValue;

#[allow(unused)]
pub struct RewardInfo {
    accumulated: u64,
    accumulated_decimals: u64,
    issue_time: u64,
    last_claim_time: u64,
    unclaimed: u64,
    unclaimed_decimals: u64,
}

impl RewardInfo {
    pub fn new_from_json_value(json: JsonValue) -> Option<RewardInfo> {
        if let JsonValue::Object(obj) = json {
            let data_value = obj.get("data")?;
            if let JsonValue::Object(data_obj) = data_value {
                let accumulated = data_obj.get("accumulated")?.as_u64()?;
                let accumulated_decimals = data_obj.get("accumulated_decimals")?.as_u64()?;
                let issue_time = data_obj.get("issue_time")?.as_u64()?;
                let last_claim_time = data_obj.get("last_claim_time")?.as_u64()?;
                let unclaimed = data_obj.get("unclaimed")?.as_u64()?;
                let unclaimed_decimals = data_obj.get("unclaimed_decimals")?.as_u64()?;
                return Some(RewardInfo {
                    accumulated,
                    accumulated_decimals,
                    issue_time,
                    last_claim_time,
                    unclaimed,
                    unclaimed_decimals,
                });
            }
        }
        None
    }

    pub fn unclaimed_gt(&self, rhs: u64) -> bool {
        self.unclaimed > rhs
    }
}
