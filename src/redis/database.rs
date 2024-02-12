use bstr::BString;
use std::collections::HashMap;
use tokio::sync::RwLock;

pub type DataKey = BString;
pub type DataValue = BString;
pub type DataBase = RwLock<HashMap<DataKey, DataValue>>;
