use crate::JwtSignOpts;
use std::io::Read;

pub fn process_gen_jwt_token(
    claims: &JwtSignOpts,
    secret_reader: &mut Box<dyn Read>,
) -> anyhow::Result<String> {
    let mut secret_buf = Vec::new();
    secret_reader.read_to_end(&mut secret_buf)?;
    let key = jsonwebtoken::EncodingKey::from_secret(&secret_buf);
    let token = jsonwebtoken::encode(&jsonwebtoken::Header::default(), claims, &key)?;
    Ok(token)
}

pub fn process_verify_jwt_token(
    secret_reader: &mut Box<dyn Read>,
    token_reader: &mut Box<dyn Read>,
) -> anyhow::Result<JwtSignOpts> {
    let mut secret_buf = Vec::new();
    secret_reader.read_to_end(&mut secret_buf)?;
    let key = jsonwebtoken::DecodingKey::from_secret(secret_buf.as_ref());
    let mut token_buf = Vec::new();
    token_reader.read_to_end(&mut token_buf)?;
    let token = std::str::from_utf8(&token_buf)?;

    let mut validation = jsonwebtoken::Validation::default();
    validation.validate_aud = false;
    println!("validation: {:?}", validation);

    let token_data = jsonwebtoken::decode::<JwtSignOpts>(token, &key, &validation)
        .map_err(|e| anyhow::anyhow!("jwt token invalid! {e}"))?;
    Ok(token_data.claims)
}
