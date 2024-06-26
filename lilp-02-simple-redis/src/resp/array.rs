use std::ops::Deref;

use bytes::{Buf, BytesMut};

use crate::{RespDecode, RespEncode, RespError, RespFrame};

use super::{calc_total_length, parse_length, BUF_CAP, CRLF_LEN};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RespArray(pub(crate) Option<Vec<RespFrame>>);

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
// pub struct RespNullArray;

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
impl RespEncode for RespArray {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        match self.0 {
            Some(frames) => {
                buf.extend_from_slice(&format!("*{}\r\n", frames.len()).into_bytes());
                for frame in frames {
                    buf.extend_from_slice(&frame.encode());
                }
            }
            None => {
                buf.extend_from_slice(b"*-1\r\n"); // RESP的空数组表示
            }
        }
        buf
    }
}

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
// - "*2\r\n$3\r\nget\r\n$5\r\nhello\r\n"
// 每个命令或数据帧都是以 \r\n （回车符后跟换行符）结束
// FIXME: need to handle incomplete
impl RespDecode for RespArray {
    const PREFIX: &'static str = "*";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        println!("buf: {:?}", String::from_utf8_lossy(&buf));
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        // 如果是空数组 null array: "*-1\r\n" parse_length 中匹配到长度为-1 len 返回值是0
        if len == 0 {
            buf.advance(end + CRLF_LEN);
            return Ok(RespArray(None));
        }
        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;

        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }

        // 将缓冲区的游标向前推进，跳过已经解析完成的部分。
        buf.advance(end + CRLF_LEN);

        let mut frames = Vec::with_capacity(len);
        for _ in 0..len {
            frames.push(RespFrame::decode(buf)?);
        }

        Ok(RespArray::new(frames))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

// - null array: "*-1\r\n"
// impl RespEncode for RespNullArray {
//     fn encode(self) -> Vec<u8> {
//         b"*-1\r\n".to_vec()
//     }
// }
//
// impl RespDecode for RespNullArray {
//     const PREFIX: &'static str = "*";
//     fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
//         extract_fixed_data(buf, "*-1\r\n", "NullArray")?;
//         Ok(RespNullArray)
//     }
//
//     fn expect_length(_buf: &[u8]) -> Result<usize, RespError> {
//         Ok(4)
//     }
// }

impl RespArray {
    pub fn new(s: impl Into<Vec<RespFrame>>) -> Self {
        RespArray(Some(s.into()))
    }
}

impl Deref for RespArray {
    type Target = Vec<RespFrame>;

    fn deref(&self) -> &Self::Target {
        // 静态空Vec<RespFrame>，供deref调用
        static EMPTY_VEC: Vec<RespFrame> = Vec::new();
        self.0.as_ref().unwrap_or(&EMPTY_VEC) // 安全返回Vec<RespFrame>引用
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::BulkString;

    use super::*;

    #[test]
    fn test_array_encode() {
        let frame: RespFrame = RespArray::new(vec![
            BulkString::new("set".to_string()).into(),
            BulkString::new("hello".to_string()).into(),
            BulkString::new("world".to_string()).into(),
        ])
        .into();
        assert_eq!(
            &frame.encode(),
            b"*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n"
        );
    }

    // #[test]
    // fn test_null_array_encode() {
    //     let frame: RespFrame = RespNullArray.into();
    //     assert_eq!(frame.encode(), b"*-1\r\n");
    // }
    //
    // #[test]
    // fn test_null_array_decode() -> Result<()> {
    //     let mut buf = BytesMut::new();
    //     buf.extend_from_slice(b"*-1\r\n");
    //
    //     let frame = RespNullArray::decode(&mut buf)?;
    //     assert_eq!(frame, RespNullArray);
    //
    //     Ok(())
    // }

    #[test]
    fn test_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello".into()]));

        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n");
        let ret = RespArray::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);

        buf.extend_from_slice(b"$6\r\nhello!\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello!".into()]));

        buf.extend_from_slice(b"*-1\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray(None));

        Ok(())
    }
}
