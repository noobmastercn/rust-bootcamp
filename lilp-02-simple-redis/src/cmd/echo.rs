use crate::cmd::{extract_args, validate_command, CommandError, CommandExecutor, Echo};
use crate::{Backend, BulkString, RespArray, RespFrame};

impl CommandExecutor for Echo {
    fn execute(self, _backend: &Backend) -> RespFrame {
        BulkString::new(self.msg.as_bytes().to_vec()).into()
    }
}

impl TryFrom<RespArray> for Echo {
    type Error = CommandError;
    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["echo"], 1)?;

        let mut args = extract_args(value, 1)?.into_iter();
        match args.next() {
            Some(RespFrame::BulkString(BulkString(Some(key)))) => Ok(Echo {
                msg: String::from_utf8(key)?,
            }),
            _ => Err(CommandError::InvalidArgument("Invalid message".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use bytes::BytesMut;

    use crate::RespDecode;

    use super::*;

    #[test]
    fn test_echo() -> Result<()> {
        let cmd = RespArray(Some(vec![
            RespFrame::BulkString(BulkString(Some(b"echo".to_vec()))),
            RespFrame::BulkString(BulkString(Some(b"hello world~".to_vec()))),
        ]));
        let cmd = Echo::try_from(cmd)?;
        assert_eq!(cmd.msg, "hello world~");

        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$4\r\necho\r\n$12\r\nhello world~\r\n");
        let frame = RespArray::decode(&mut buf)?;
        let result: Echo = frame.try_into()?;
        assert_eq!(result.msg, "hello world~");

        Ok(())
    }
}
