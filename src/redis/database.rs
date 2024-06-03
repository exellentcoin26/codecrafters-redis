use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use bstr::BString;
use tokio::sync::RwLock;

pub type DataKey = BString;
pub type DataValue = BString;
pub type Database = RwLock<HashMap<DataKey, DataValueWithParams>>;

#[derive(Debug, PartialEq, Eq)]
pub struct DataValueWithParams {
    pub value: DataValue,
    pub created_at: SystemTime,
    pub expiry: Option<Duration>,
}

impl DataValueWithParams {
    pub fn new(value: DataValue, expiry: Option<Duration>) -> Self {
        Self {
            value,
            created_at: SystemTime::now(),
            expiry,
        }
    }
}

impl From<DataValue> for DataValueWithParams {
    fn from(value: DataValue) -> Self {
        Self::new(value, None)
    }
}
