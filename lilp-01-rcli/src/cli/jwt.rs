use crate::{get_reader, process_gen_jwt_token, process_verify_jwt_token, CmdExector};
use clap::Parser;
use enum_dispatch::enum_dispatch;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::Write;

use super::verify_file;

#[derive(Debug, Parser)]
#[enum_dispatch(CmdExector)]
pub enum JwtSubCommand {
    #[command(about = "Sign a json web token(jwt)")]
    Sign(JwtSignOpts),
    #[command(about = "Verify a json web token(jwt)")]
    Verify(JwtVerifyOpts),
}

#[derive(Debug, Serialize, Deserialize, Parser)]
pub struct JwtSignOpts {
    #[arg(short, long)]
    pub sub: String,
    #[arg(short, long)]
    pub aud: String,
    #[arg(short, long, value_parser = verify_exp)]
    pub exp: usize,
}

pub fn verify_exp(exp: &str) -> Result<usize, &'static str> {
    let re = Regex::new(r"^(\d+)([dhms])$").unwrap();
    if let Some(caps) = re.captures(exp) {
        let quantity = caps
            .get(1)
            .unwrap()
            .as_str()
            .parse::<usize>()
            .map_err(|_| "Invalid number")?;
        let unit = caps.get(2).unwrap().as_str();
        match unit {
            "d" => Ok(get_epoch() + quantity * 86_400), // 天转秒
            "h" => Ok(get_epoch() + quantity * 3_600),  // 小时转秒
            "m" => Ok(get_epoch() + quantity * 60),     // 分钟转秒
            "s" => Ok(get_epoch() + quantity),          // 秒
            _ => Err("Invalid unit. Use d, h, m, s."),
        }
    } else {
        Err("Invalid format. Use <number><unit>, e.g., 14d, 24h.")
    }
}

fn get_epoch() -> usize {
    let start = std::time::SystemTime::now();
    start
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
}

#[derive(Debug, Parser)]
pub struct JwtVerifyOpts {
    #[arg(short, long, value_parser = verify_file, default_value = "-")]
    pub token: String,
}

impl CmdExector for JwtSignOpts {
    async fn execute(self) -> anyhow::Result<()> {
        // 从fixtures/jwt-secret.txt中读取密钥
        let mut secret_reader = get_reader("fixtures/jwt-secret.txt")?;
        let token = process_gen_jwt_token(&self, &mut secret_reader)?;
        // 写入到文件
        let mut token_writer = std::fs::File::create("fixtures/jwt-token.txt")?;
        token_writer.write_all(token.as_bytes())?;
        println!("json web token: {}", token);
        Ok(())
    }
}

impl CmdExector for JwtVerifyOpts {
    async fn execute(self) -> anyhow::Result<()> {
        // 从fixtures/jwt-secret.txt中读取密钥
        let mut secret_reader = get_reader("fixtures/jwt-secret.txt")?;
        let mut token_reader = get_reader(&self.token)?;
        let verified = process_verify_jwt_token(&mut secret_reader, &mut token_reader)?;
        println!("json web token verified: {:?}", verified);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_sign() {
        let opts = JwtSignOpts {
            sub: "test".into(),
            aud: "test".into(),
            exp: get_epoch() + 3600,
        };
        let _x = opts.execute().await.unwrap();
    }

    #[tokio::test]
    async fn test_jwt_verify() {
        let opts = JwtVerifyOpts {
            token: "fixtures/jwt-token.txt".into(),
        };
        let _x = opts.execute().await.unwrap();
    }
}
