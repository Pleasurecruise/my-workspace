use crate::UgosError;
use crate::types::ApiResponse;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct CheckRequest<'a> {
    username: &'a str,
}

#[derive(Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    password: String,
    keepalive: bool,
    otp: bool,
    is_simple: bool,
}

#[derive(Deserialize)]
struct LoginData {
    token: String,
}

pub(crate) async fn login(
    http: &reqwest::Client,
    base_url: &str,
    username: &str,
    password: &str,
    client_id: &str,
    device_id: &str,
    client_version: &str,
) -> Result<String, UgosError> {
    let check = http
        .post(format!("{base_url}/v1/verify/check"))
        .json(&CheckRequest { username })
        .send()
        .await?
        .error_for_status()?;
    let header = check
        .headers()
        .get("x-rsa-token")
        .ok_or_else(|| UgosError::Encryption("verify/check omitted x-rsa-token".to_owned()))?
        .to_str()
        .map_err(|error| UgosError::Encryption(format!("invalid x-rsa-token header: {error}")))?;
    let pem = STANDARD
        .decode(header)
        .map_err(|error| UgosError::Encryption(format!("invalid RSA token encoding: {error}")))?;
    let pem = String::from_utf8(pem)
        .map_err(|error| UgosError::Encryption(format!("RSA public key is not UTF-8: {error}")))?;
    let public_key = match RsaPublicKey::from_pkcs1_pem(&pem) {
        Ok(public_key) => public_key,
        Err(pkcs1_error) => match RsaPublicKey::from_public_key_pem(&pem) {
            Ok(public_key) => public_key,
            Err(spki_error) => {
                let relabeled = pem
                    .replace("BEGIN RSA PUBLIC KEY", "BEGIN PUBLIC KEY")
                    .replace("END RSA PUBLIC KEY", "END PUBLIC KEY");
                RsaPublicKey::from_public_key_pem(&relabeled).map_err(|relabeled_error| {
                    UgosError::Encryption(format!(
                        "could not parse RSA public key: {pkcs1_error}; {spki_error}; {relabeled_error}"
                    ))
                })?
            }
        },
    };
    let encrypted = public_key
        .encrypt(
            &mut rsa::rand_core::OsRng,
            Pkcs1v15Encrypt,
            password.as_bytes(),
        )
        .map_err(|error| UgosError::Encryption(format!("could not encrypt password: {error}")))?;
    let response = http
        .post(format!("{base_url}/v1/verify/login"))
        .json(&LoginRequest {
            username,
            password: STANDARD.encode(encrypted),
            keepalive: true,
            otp: true,
            is_simple: true,
        })
        .header("Accept", "application/json, text/plain, */*")
        .header("UG-Agent", "PC/WEB")
        .header("Client-Id", client_id)
        .header("UG-Client-Id", device_id)
        .header("Client-Version", client_version)
        .header("Cache-Control", "no-cache")
        .header("X-Specify-Language", "en-US")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(ApiResponse::<LoginData>::decode(&response, "verify/login")?.token)
}

#[cfg(test)]
#[path = "../tests/unit/auth.rs"]
mod tests;
