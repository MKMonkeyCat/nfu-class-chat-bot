use chrono::{Duration as ChronoDuration, Utc};
use config::GoogleCalendarConfig;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::Client;
use tokio::fs;

use super::types::{
    CachedGoogleToken, GoogleJwtClaims, GoogleServiceAccount, GoogleTokenRequest,
    GoogleTokenResponse,
};

const GOOGLE_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";

pub(crate) async fn get_or_refresh_access_token(
    reqwest: &Client,
    gcal: &GoogleCalendarConfig,
    token_cache: &mut Option<CachedGoogleToken>,
) -> Result<String, String> {
    if let Some(cached) = token_cache {
        if cached.expires_at > Utc::now() + ChronoDuration::seconds(30) {
            return Ok(cached.access_token.clone());
        }
    }

    let account_json = fs::read_to_string(&gcal.service_account_json_path)
        .await
        .map_err(|err| format!("read service account json failed: {err}"))?;

    let account: GoogleServiceAccount = serde_json::from_str(&account_json)
        .map_err(|err| format!("parse service account json failed: {err}"))?;

    let now = Utc::now();
    let claims = GoogleJwtClaims {
        iss: account.client_email.clone(),
        scope: GOOGLE_SCOPE.to_string(),
        aud: account.token_uri.clone(),
        iat: now.timestamp(),
        exp: (now + ChronoDuration::minutes(59)).timestamp(),
    };

    let key = EncodingKey::from_rsa_pem(account.private_key.as_bytes())
        .map_err(|err| format!("build rsa key failed: {err}"))?;

    let assertion = encode(&Header::new(Algorithm::RS256), &claims, &key)
        .map_err(|err| format!("encode jwt failed: {err}"))?;

    let token_response = reqwest
        .post(&account.token_uri)
        .form(&GoogleTokenRequest {
            grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer",
            assertion: &assertion,
        })
        .send()
        .await
        .map_err(|err| format!("request token failed: {err}"))?
        .error_for_status()
        .map_err(|err| format!("token status failed: {err}"))?
        .json::<GoogleTokenResponse>()
        .await
        .map_err(|err| format!("parse token response failed: {err}"))?;

    let expires_in = token_response.expires_in.max(60);
    let cached = CachedGoogleToken {
        access_token: token_response.access_token.clone(),
        expires_at: Utc::now() + ChronoDuration::seconds((expires_in - 30) as i64),
    };
    *token_cache = Some(cached);

    Ok(token_response.access_token)
}
