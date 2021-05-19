use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PositionIntent {
    pub id: String,
    pub strategy: String,
    pub timestamp: DateTime<Utc>,
    pub ticker: String,
    pub qty: i32,
}
