use josekit::jwk::JwkSet;
use josekit::jwt::{
    JwtPayload, JwtPayloadValidator, decode_with_verifier, decode_with_verifier_in_jwk_set,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant, sleep};

/// Response received from the device code endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Successful token response.
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub id_token: String,
}

/// Error response from the token endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenErrorResponse {
    pub error: String,
    pub error_description: String,
}

/// Represents the result of trying to get a token.
#[derive(Debug)]
pub enum TokenResult {
    Success(TokenResponse),
    Error { error: String, description: String },
}

/// Fetches the device code from the authorization server.
pub async fn get_device_code(
    device_url: &str,
    client_id: &str,
) -> Result<DeviceCodeResponse, reqwest::Error> {
    let client = Client::new();
    let params = [("client_id", client_id)];

    let response = client.post(device_url).form(&params).send().await?;

    // If the response is not successful, return the error.
    if !response.status().is_success() {
        return Err(response.error_for_status().unwrap_err());
    }

    let device_code_resp = response.json::<DeviceCodeResponse>().await?;
    Ok(device_code_resp)
}

/// Attempts to exchange the device code for tokens.
pub async fn get_token(
    token_url: &str,
    client_id: &str,
    device_code: &str,
) -> Result<TokenResult, reqwest::Error> {
    let client = Client::new();
    let params = [
        ("client_id", client_id),
        ("device_code", device_code),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
    ];

    let response = client.post(token_url).form(&params).send().await?;

    if response.status().is_success() {
        let token_response = response.json::<TokenResponse>().await?;
        Ok(TokenResult::Success(token_response))
    } else {
        let error_response = response.json::<TokenErrorResponse>().await?;
        Ok(TokenResult::Error {
            error: error_response.error,
            description: error_response.error_description,
        })
    }
}

/// Polls the token endpoint until a token is obtained or the device code expires.
pub async fn poll_for_token(
    token_url: &str,
    client_id: &str,
    device_code_response: &DeviceCodeResponse,
) -> Result<TokenResponse, String> {
    let end_time = Instant::now() + Duration::from_secs(device_code_response.expires_in);

    while Instant::now() < end_time {
        println!("Polling for token...");
        match get_token(token_url, client_id, &device_code_response.device_code).await {
            Ok(TokenResult::Success(token)) => return Ok(token),
            Ok(TokenResult::Error { error, description }) => {
                // If error isn't "authorization_pending", then exit.
                if error != "authorization_pending" {
                    return Err(description);
                }
            }
            Err(e) => return Err(e.to_string()),
        }
        sleep(Duration::from_secs(device_code_response.interval)).await;
    }

    Err("Authorization timed out".to_string())
}

pub async fn validate_jwt(
    token: &str,
    client_id: &str,
    issuer_url: &str,
    jwks_url: &str,
) -> Result<JwtPayload, String> {
    // Fetch the JWKS from the URL.
    let jwks_response = reqwest::get(jwks_url)
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    // Parse the JWKS from the JSON bytes.
    let jwks = JwkSet::from_bytes(jwks_response.as_bytes()).map_err(|e| e.to_string())?;

    // Build a JWT validator step by step.
    let mut validator = JwtPayloadValidator::new();
    validator.set_issuer(issuer_url);
    validator.set_audience(client_id);

    // Assuming you have a JWT string called 'jwt' and a verifier 'verifier'
    let (payload, _) = decode_with_verifier_in_jwk_set(token, &jwks, validator);
    // Validate the payload
    validator.validate(&payload);

    // // Validate the token using the JWKS.
    // let payload = validator
    //     .validate(token, &jwks)
    //     .map_err(|e| e.to_string())?;

    Ok(payload)
}
