use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use bstr::{BStr, BString, ByteSlice};

use super::database::{DataValueWithParams, Database};
use crate::redis::{database, RespValue};

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Ping,
    Echo {
        value: BString,
    },
    Set {
        key: database::DataKey,
        value: database::DataValue,
        opts: SetOptions,
    },
    Get {
        key: database::DataKey,
    },
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct SetOptions {
    expiry: Option<Duration>,
}

impl Command {
    pub fn from_wire(wire: &'_ [u8]) -> Result<Self> {
        let value = RespValue::from_resp(wire)?;
        CommandParser::parse_single_command(value)
    }

    pub async fn execute(self, database: &Database) -> Result<RespValue> {
        Ok(match self {
            Command::Ping => RespValue::SimpleString("PONG".to_string()),
            Command::Echo { value } => RespValue::BulkString(value),
            Command::Set { key, value, opts } => {
                let mut database = database.write().await;
                let value = DataValueWithParams::new(value, opts.expiry);
                database.insert(key, value);
                RespValue::SimpleString("OK".to_string())
            }
            Command::Get { key } => Self::get(database, key).await,
        })
    }

    async fn get(database: &Database, key: database::DataKey) -> RespValue {
        let database = database.read().await;

        let Some(DataValueWithParams {
            value,
            created_at,
            expiry,
        }) = database.get(&key)
        else {
            return RespValue::Nil;
        };

        if let Some(expiry) = expiry {
            if SystemTime::now()
                .duration_since(*created_at)
                .map(|d| d < *expiry)
                .unwrap_or(true)
            {
                return RespValue::Nil;
            }
        }

        RespValue::BulkString(value.clone())
    }
}

pub struct CommandParser;

impl CommandParser {
    fn parse_single_command(value: RespValue) -> Result<Command> {
        match value {
            RespValue::SimpleString(command) => Self::from_simple_string(&command),
            RespValue::BulkString(command) => Self::from_bulk_string(command.as_bstr()),
            RespValue::Array(arr) => Self::from_array(arr),
            _ => bail!("unsupported resp value to parse command from `{:?}`", value),
        }
    }

    fn from_bulk_string(command: &BStr) -> Result<Command> {
        let command = std::str::from_utf8(command)
            .context("command name contains invalid utf-8 characters")?;
        Self::from_simple_string(command)
    }

    fn from_simple_string(command: &'_ str) -> Result<Command> {
        log::debug!("Command: {}", command.to_lowercase().as_str());
        Ok(match command.to_lowercase().as_str() {
            "ping" => Command::Ping,
            _ => bail!("unexpected command `{}`", command),
        })
    }

    fn from_array(array: Box<[RespValue]>) -> Result<Command> {
        let mut array = array.into_vec().into_iter();
        let command = match array
            .next()
            .context("input array for command cannot be empty")?
        {
            RespValue::SimpleString(command) => command,
            RespValue::BulkString(command) => String::try_from(command)?,
            _ => bail!("unexpeced data type as command name"),
        };

        Ok(match command.to_lowercase().as_str() {
            "ping" => Command::Ping,
            "echo" => Self::take_as_echo_args(&mut array)?,
            "get" => Self::take_as_get_args(&mut array)?,
            "set" => Self::take_as_set_args(&mut array)?,
            _ => bail!("unexpected command `{}`", command),
        })
    }

    fn take_as_echo_args(args: &mut impl Iterator<Item = RespValue>) -> Result<Command> {
        let value = Self::take_byte_string_argument(args)?;
        Ok(Command::Echo { value })
    }

    fn take_as_get_args(args: &mut impl Iterator<Item = RespValue>) -> Result<Command> {
        let key = Self::take_byte_string_argument(args)?;
        Ok(Command::Get { key })
    }

    fn take_as_set_args(args: &mut impl Iterator<Item = RespValue>) -> Result<Command> {
        let key = Self::take_byte_string_argument(&mut *args)?;
        let value = Self::take_byte_string_argument(&mut *args)?;

        let mut opts = SetOptions::default();

        while let Some(opt) = Self::take_optional_byte_string_argument(args)? {
            match std::str::from_utf8(&opt)
                .context("command key-value pair key argument contains invalid utf-8")?
            {
                "px" => {
                    opts.expiry = Some(Duration::from_millis(Self::take_string_and_parse_u64(
                        args,
                    )?))
                }
                _ => bail!("unexpected option for set command: {}", opt),
            }
        }

        Ok(Command::Set { key, value, opts })
    }

    fn take_byte_string_argument(args: &mut impl Iterator<Item = RespValue>) -> Result<BString> {
        Ok(
            match args.next().context("command expected another argument")? {
                RespValue::SimpleString(value) => BString::from(value),
                RespValue::BulkString(command) => command,
                _ => bail!("unexpected data type as command argument"),
            },
        )
    }

    fn take_optional_byte_string_argument(
        args: &mut impl Iterator<Item = RespValue>,
    ) -> Result<Option<BString>> {
        match args.next() {
            Some(a) => Self::take_byte_string_argument(&mut std::iter::once(a)).map(Some),
            None => Ok(None),
        }
    }

    fn take_string_and_parse_u64(args: &mut impl Iterator<Item = RespValue>) -> Result<u64> {
        let value = Self::take_byte_string_argument(args)?;
        std::str::from_utf8(&value)
            .context("integer value contains invalid utf-8")?
            .parse::<u64>()
            .context("string is not valid integer value")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bstr::BString;

    use super::{Command, SetOptions};

    #[test]
    fn parse_ping_command() {
        let command = Command::from_wire(b"+PING\r\n").unwrap();
        assert_eq!(command, Command::Ping);
    }

    #[test]
    fn parse_echo_simple_string() {
        let command = Command::from_wire(b"*2\r\n+ECHO\r\n+Hello, World!\r\n").unwrap();
        assert_eq!(
            command,
            Command::Echo {
                value: BString::from("Hello, World!")
            }
        )
    }

    #[test]
    fn parse_echo_bulk_string() {
        let command = Command::from_wire(b"*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n").unwrap();
        assert_eq!(
            command,
            Command::Echo {
                value: BString::from("hey")
            }
        )
    }

    #[test]
    fn parse_get_bulk_string() {
        let command = Command::from_wire(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n").unwrap();
        assert_eq!(
            command,
            Command::Get {
                key: BString::from("foo")
            }
        )
    }

    #[test]
    fn parse_set_bulk_string() {
        let command = Command::from_wire(b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").unwrap();
        assert_eq!(
            command,
            Command::Set {
                key: BString::from("foo"),
                value: BString::from("bar"),
                opts: SetOptions::default(),
            }
        )
    }

    #[test]
    fn parse_set_with_expiry() {
        let command = Command::from_wire(
            b"*5\r\n$3\r\nset\r\n$9\r\nelephants\r\n$6\r\nhorses\r\n$2\r\npx\r\n$3\r\n100\r\n",
        )
        .unwrap();
        assert_eq!(
            command,
            Command::Set {
                key: BString::from("elephants"),
                value: BString::from("horses"),
                opts: SetOptions {
                    expiry: Some(Duration::from_millis(100))
                }
            }
        )
    }
}
