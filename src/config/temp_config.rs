use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TempConfigJson {
    temp_pswd: HashMap<String, String>,
}

impl TempConfigJson {
    pub(crate) fn take_pswd(&mut self, id: &String) -> Option<String> {
        Some(self.temp_pswd.get_mut(id)?.drain(..).collect())
    }
}
