use crate::{
    get_content, get_reader, process_text_decrypt, process_text_encrypt, process_text_key_generate,
    process_text_sign, process_text_verify, CmdExector,
};

use super::{verify_file, verify_path};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use clap::Parser;
use enum_dispatch::enum_dispatch;
use std::{fmt, path::PathBuf, str::FromStr};
use tokio::fs;

#[derive(Debug, Parser)]
#[enum_dispatch(CmdExector)]
pub enum TextSubCommand {
    #[command(about = "Sign a text with a private/session key and return a signature")]
    Sign(TextSignOpts),
    #[command(about = "Verify a signature with a public/session key")]
    Verify(TextVerifyOpts),
    #[command(about = "Generate a random blake3 key or ed25519 key pair")]
    Generate(KeyGenerateOpts),
    #[command(about = "Encrypt a text with a public key")]
    Encrypt(TextEncryptOpts),
    #[command(about = "Decrypt a text with a private key")]
    Decrypt(TextDecryptOpts),
}

#[derive(Debug, Parser)]
pub struct TextSignOpts {
    #[arg(short, long, value_parser = verify_file, default_value = "-")]
    pub input: String,
    #[arg(short, long, value_parser = verify_file)]
    pub key: String,
    #[arg(long, default_value = "blake3", value_parser = parse_text_sign_format)]
    pub format: TextSignFormat,
}

#[derive(Debug, Parser)]
pub struct TextVerifyOpts {
    #[arg(short, long, value_parser = verify_file, default_value = "-")]
    pub input: String,
    #[arg(short, long, value_parser = verify_file)]
    pub key: String,
    #[arg(long)]
    pub sig: String,
    #[arg(long, default_value = "blake3", value_parser = parse_text_sign_format)]
    pub format: TextSignFormat,
}

#[derive(Debug, Parser)]
pub struct KeyGenerateOpts {
    #[arg(long, default_value = "blake3", value_parser = parse_text_sign_format)]
    pub format: TextSignFormat,
    #[arg(short, long, value_parser = verify_path)]
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub enum TextSignFormat {
    Blake3,
    Ed25519,
}

#[derive(Debug, Parser)]
pub struct TextEncryptOpts {
    /// 输入需要加密的内容
    #[arg(short, long, value_parser = verify_file, default_value = "-")]
    pub input: String,
    /// 输入密钥文件路径 32位
    #[arg(short, long, value_parser = verify_file)]
    pub key: String,
}

#[derive(Debug, Parser)]
pub struct TextDecryptOpts {
    /// 输入需要解密的内容路径
    #[arg(short, long, value_parser = verify_file, default_value = "-")]
    pub input: String,
    /// 输入密钥文件路径
    #[arg(short, long, value_parser = verify_file)]
    pub key: String,
}

fn parse_text_sign_format(format: &str) -> Result<TextSignFormat, anyhow::Error> {
    format.parse()
}

impl FromStr for TextSignFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blake3" => Ok(TextSignFormat::Blake3),
            "ed25519" => Ok(TextSignFormat::Ed25519),
            _ => Err(anyhow::anyhow!("Invalid format")),
        }
    }
}

impl From<TextSignFormat> for &'static str {
    fn from(format: TextSignFormat) -> Self {
        match format {
            TextSignFormat::Blake3 => "blake3",
            TextSignFormat::Ed25519 => "ed25519",
        }
    }
}

impl fmt::Display for TextSignFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl CmdExector for TextSignOpts {
    async fn execute(self) -> anyhow::Result<()> {
        let mut reader = get_reader(&self.input)?;
        let key = get_content(&self.key)?;
        let sig = process_text_sign(&mut reader, &key, self.format)?;
        // base64 output
        let encoded = URL_SAFE_NO_PAD.encode(sig);
        println!("{}", encoded);
        Ok(())
    }
}

impl CmdExector for TextVerifyOpts {
    async fn execute(self) -> anyhow::Result<()> {
        let mut reader = get_reader(&self.input)?;
        let key = get_content(&self.key)?;
        let decoded = URL_SAFE_NO_PAD.decode(&self.sig)?;
        let verified = process_text_verify(&mut reader, &key, &decoded, self.format)?;
        if verified {
            println!("✓ Signature verified");
        } else {
            println!("⚠ Signature not verified");
        }
        Ok(())
    }
}

impl CmdExector for KeyGenerateOpts {
    async fn execute(self) -> anyhow::Result<()> {
        let key = process_text_key_generate(self.format)?;
        for (k, v) in key {
            fs::write(self.output_path.join(k), v).await?;
        }
        Ok(())
    }
}

impl CmdExector for TextEncryptOpts {
    async fn execute(self) -> anyhow::Result<()> {
        // 获取用户输入内容
        let mut reader = get_reader(&self.input)?;
        // 获取用户输入的key地址
        let key = get_content(&self.key)?;
        // encrypt
        // let sig = process_text_sign(&mut reader, &key, self.format)?;
        let ciphertext = process_text_encrypt(&mut reader, &key)?;
        // base64 output
        let encoded = URL_SAFE_NO_PAD.encode(ciphertext);
        println!(" 加密文本： {}", encoded);
        Ok(())
    }
}

impl CmdExector for TextDecryptOpts {
    async fn execute(self) -> anyhow::Result<()> {
        // 获取用户输入内容
        let mut reader = get_reader(&self.input)?;
        // 获取用户输入的key地址
        let key = get_content(&self.key)?;
        // decrypt
        let plaintext = process_text_decrypt(&mut reader, &key)?;
        println!(" 解密文本：{}", String::from_utf8_lossy(&plaintext));
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use chacha20poly1305::{
        aead::{Aead, AeadCore, KeyInit, OsRng},
        ChaCha20Poly1305, Key, Nonce,
    };

    #[test]
    fn test_chacha20poly1305() -> anyhow::Result<()> {
        // 生成32位密码用来生成key
        // 保存key到本地。 把 nonce和ciphertext 一起保存 用于解密

        let key_bytes = [
            149, 118, 201, 102, 159, 190, 190, 211, 218, 86, 10, 209, 206, 209, 248, 243, 15, 242,
            38, 31, 117, 175, 46, 180, 96, 68, 3, 136, 196, 66, 247, 146,
        ];
        let x = Key::from_slice(&key_bytes);

        let cipher = ChaCha20Poly1305::new(x);
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng); // 96-bits; unique per message

        println!("nonce: {:?}", nonce.len());

        println!("Nonce: {:?}", nonce);

        let ciphertext = cipher
            .encrypt(&nonce, b"plaintext message".as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption error: {:?}", e))?;
        let plaintext = cipher
            .decrypt(&nonce, &*ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption error: {:?}", e))?;
        assert_eq!(&plaintext, b"plaintext message");
        Ok(())
    }
}
