use super::{model::RespValue, CRLF};

pub(in crate::redis) struct RespSerializer;

impl RespSerializer {
    pub(in crate::redis) fn serialize(
        f: &mut dyn std::io::Write,
        value: &RespValue,
    ) -> std::io::Result<()> {
        match value {
            RespValue::SimpleString(value) => write!(f, "+{}{}", value, CRLF),
            RespValue::BulkString(value) => {
                write!(f, "${}{}", value.len(), CRLF)?;
                f.write_all(value)?;
                write!(f, "{}", CRLF)
            }
            RespValue::Array(arr) => {
                write!(f, "*{}{}", arr.len(), CRLF)?;
                arr.iter().try_for_each(|v| Self::serialize(&mut *f, v))
            }
        }
    }
}
