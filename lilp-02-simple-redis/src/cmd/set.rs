use crate::cmd::{extract_args, validate_command, CommandError, CommandExecutor, Sadd, Sismember};
use crate::{BulkString, RespArray, RespEncode, RespFrame};

impl CommandExecutor for Sadd {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        let count = backend.sadd(self.key, self.members) as i64;
        let frame: RespFrame = count.into();
        println!("send : {:?}", String::from_utf8(frame.clone().encode()));
        frame
    }
}

impl CommandExecutor for Sismember {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        let exists = backend.sismember(&self.key, &self.member);
        if exists {
            return RespFrame::Integer(1);
        }
        RespFrame::Integer(0)
    }
}

// SADD key member [member ...]
// *4\r\n$4\r\nSADD\r\n$3\r\nkey\r\n$2\r\nm1\r\n$2\r\nm2\r\n
impl TryFrom<RespArray> for Sadd {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        let n_args = value.len() - 1;
        validate_command(&value, &["sadd"], n_args)?;
        let mut args = extract_args(value, 1)?.into_iter();
        // // 迭代 args 第一个是 key 后面的是 member 把后面的都放到 members 里面
        let key = match args.next() {
            Some(RespFrame::BulkString(BulkString(Some(key)))) => String::from_utf8(key)?,
            _ => {
                return Err(CommandError::InvalidArgument(
                    "Invalid Sadd key".to_string(),
                ))
            }
        };
        let mut members = Vec::with_capacity(n_args);
        while let Some(RespFrame::BulkString(member)) = args.next() {
            members.push(member);
        }
        Ok(Sadd { key, members })
    }
}

// SISMEMBER key member
// *3\r\n$9\r\nSISMEMBER\r\n$3\r\nkey\r\n$2\r\nm1\r\n
impl TryFrom<RespArray> for Sismember {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["sismember"], 2)?;
        let mut args = extract_args(value, 1)?.into_iter();
        match (args.next(), args.next()) {
            (
                Some(RespFrame::BulkString(BulkString(Some(key)))),
                Some(RespFrame::BulkString(member)),
            ) => Ok(Sismember {
                key: String::from_utf8(key)?,
                member,
            }),
            _ => Err(CommandError::InvalidArgument(
                "Invalid Sismember key or member".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::RespDecode;

    use super::*;

    #[test]
    fn test_sadd_from_resp_array() -> anyhow::Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*4\r\n$4\r\nSADD\r\n$4\r\nlilp\r\n$2\r\nm1\r\n$2\r\nm2\r\n");

        let frame = RespArray::decode(&mut buf)?;

        let result: Sadd = frame.try_into()?;
        assert_eq!(result.key, "lilp");
        assert_eq!(
            result.members,
            [BulkString::new("m1"), BulkString::new("m2")]
        );

        Ok(())
    }

    #[test]
    fn test_sismember_from_resp_array() -> anyhow::Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*3\r\n$9\r\nSISMEMBER\r\n$4\r\nlilp\r\n$2\r\nm1\r\n");

        let frame = RespArray::decode(&mut buf)?;

        let result: Sismember = frame.try_into()?;
        assert_eq!(result.key, "lilp");
        assert_eq!(result.member, BulkString::new("m1"));

        Ok(())
    }

    #[test]
    fn test_sadd_execute() -> anyhow::Result<()> {
        let backend = crate::Backend::new();
        let sadd = Sadd {
            key: "lilp".to_string(),
            members: vec![BulkString::new("m1"), BulkString::new("m2")],
        };
        let result = sadd.execute(&backend);

        let backend_set_lilp = backend.set.get("lilp").unwrap();

        assert_eq!(backend_set_lilp.len(), 2);
        assert_eq!(result, RespFrame::Integer(2));

        Ok(())
    }

    #[test]
    fn test_sismember_execute() -> anyhow::Result<()> {
        let backend = crate::Backend::new();
        let sadd = Sadd {
            key: "lilp".to_string(),
            members: vec![BulkString::new("m1"), BulkString::new("m2")],
        };
        sadd.execute(&backend);

        let sismember = Sismember {
            key: "lilp".to_string(),
            member: BulkString::new("m1"),
        };
        let result = sismember.execute(&backend);
        assert_eq!(result, RespFrame::Integer(1));

        let sismember = Sismember {
            key: "lilp".to_string(),
            member: BulkString::new("m3"),
        };
        let result = sismember.execute(&backend);
        assert_eq!(result, RespFrame::Integer(0));

        Ok(())
    }
}
