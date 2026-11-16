//! Firebase ID tokens are accepted only for the explicitly configured environment.
use crate::api::ApiError;
use axum::http::StatusCode;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use serde::Deserialize;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const KEYS: &str =
    "https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com";
#[derive(Clone)]
pub struct FirebaseVerifier {
    project: String,
    client: reqwest::Client,
    cache: Arc<Mutex<KeyCache>>,
}
struct KeyCache {
    keys: Option<JwkSet>,
    expires: Instant,
    retry_after: Instant,
}
#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
    iat: u64,
    auth_time: u64,
    firebase: FirebaseClaims,
}
#[derive(Debug, Deserialize)]
struct FirebaseClaims {
    sign_in_provider: String,
}
fn denied() -> ApiError {
    ApiError(
        StatusCode::UNAUTHORIZED,
        "unauthorized",
        "Online access could not be verified".into(),
    )
}
impl FirebaseVerifier {
    pub fn new(project: String) -> Result<Self, Box<dyn std::error::Error>> {
        if project.len() < 6
            || project.len() > 30
            || !project
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err("Invalid PUZZGEN_FIREBASE_PROJECT_ID".into());
        }
        Ok(Self {
            project,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(4))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            cache: Arc::new(Mutex::new(KeyCache {
                keys: None,
                expires: Instant::now(),
                retry_after: Instant::now(),
            })),
        })
    }
    pub fn project(&self) -> &str {
        &self.project
    }
    pub async fn verify(&self, token: &str) -> Result<String, ApiError> {
        if token.len() > 8192 {
            return Err(denied());
        }
        let header = decode_header(token).map_err(|_| denied())?;
        if header.alg != Algorithm::RS256 {
            return Err(denied());
        }
        let kid = header.kid.ok_or_else(denied)?;
        if kid.len() > 128 {
            return Err(denied());
        }
        // One bounded key fetch per cache interval, including unknown-key attacks.
        let mut cache = self.cache.lock().await;
        if cache.keys.is_none() || Instant::now() >= cache.expires {
            if Instant::now() < cache.retry_after {
                return Err(denied());
            }
            cache.retry_after = Instant::now() + Duration::from_secs(30);
            let mut response = self.client.get(KEYS).send().await.map_err(|_| denied())?;
            if !response.status().is_success()
                || response.content_length().is_some_and(|n| n > 65536)
            {
                return Err(denied());
            }
            let max_age = response
                .headers()
                .get(reqwest::header::CACHE_CONTROL)
                .and_then(|h| h.to_str().ok())
                .and_then(|h| {
                    h.split(',')
                        .find_map(|v| v.trim().strip_prefix("max-age=")?.parse::<u64>().ok())
                })
                .unwrap_or(300)
                .min(3600);
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| denied())? {
                if bytes.len() + chunk.len() > 65536 {
                    return Err(denied());
                }
                bytes.extend_from_slice(&chunk);
            }
            let keys = serde_json::from_slice::<JwkSet>(&bytes).map_err(|_| denied())?;
            if keys.keys.len() > 16 {
                return Err(denied());
            }
            cache.keys = Some(keys);
            cache.expires = Instant::now() + Duration::from_secs(max_age);
        }
        let key = cache
            .keys
            .as_ref()
            .and_then(|keys| keys.find(&kid))
            .ok_or_else(denied)?;
        let key = DecodingKey::from_jwk(key).map_err(|_| denied())?;
        drop(cache);
        verify_with_key(token, &self.project, &key)
    }
}
fn verify_with_key(token: &str, project: &str, key: &DecodingKey) -> Result<String, ApiError> {
    let mut rules = Validation::new(Algorithm::RS256);
    rules.leeway = 30;
    rules.set_audience(&[project]);
    rules.set_issuer(&[format!("https://securetoken.google.com/{project}")]);
    rules.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);
    let c = decode::<Claims>(token, key, &rules)
        .map_err(|_| denied())?
        .claims;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| denied())?
        .as_secs();
    if c.sub.is_empty()
        || c.sub.len() > 128
        || c.iat > now + 30
        || c.auth_time > now + 30
        || c.auth_time > c.iat
        || c.exp <= c.iat
        || c.firebase.sign_in_provider != "anonymous"
    {
        return Err(denied());
    }
    Ok(c.sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode, jwk::Jwk};
    use serde_json::{Value, json};
    fn fixture() -> (Value, DecodingKey) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let key: Jwk = serde_json::from_str(include_str!(
            "../tests/fixtures/firebase/test-only-public.json"
        ))
        .unwrap();
        (
            json!({"sub":"garden-fixture","aud":"beacon-dev","iss":"https://securetoken.google.com/beacon-dev","exp":now+3600,"iat":now-5,"auth_time":now-10,"firebase":{"sign_in_provider":"anonymous"}}),
            DecodingKey::from_jwk(&key).unwrap(),
        )
    }
    fn token(c: &Value) -> String {
        encode(
            &Header::new(Algorithm::RS256),
            c,
            &EncodingKey::from_rsa_der(include_bytes!(
                "../tests/fixtures/firebase/test-only-private.der"
            )),
        )
        .unwrap()
    }
    #[test]
    fn signed_guest_is_accepted_only_in_its_environment() {
        let (claims, key) = fixture();
        let jwt = token(&claims);
        assert_eq!(
            verify_with_key(&jwt, "beacon-dev", &key).unwrap(),
            "garden-fixture"
        );
        assert!(verify_with_key(&jwt, "beacon-production", &key).is_err());
    }
    #[test]
    fn rejects_bad_claims_and_non_anonymous_providers() {
        let (base, key) = fixture();
        for (field, bad) in [
            ("iss", json!("https://evil.example")),
            ("aud", json!("other-project")),
            ("sub", json!("")),
            ("exp", json!(1)),
            ("iat", json!(u64::MAX - 30)),
            ("auth_time", json!(u64::MAX - 30)),
            ("firebase", json!({"sign_in_provider":"password"})),
        ] {
            let mut claims = base.clone();
            claims[field] = bad;
            assert!(
                verify_with_key(&token(&claims), "beacon-dev", &key).is_err(),
                "{field}"
            );
        }
        let mut no_auth = base;
        no_auth.as_object_mut().unwrap().remove("auth_time");
        assert!(verify_with_key(&token(&no_auth), "beacon-dev", &key).is_err());
    }
    #[test]
    fn rejects_unsigned_and_hmac_tokens() {
        let (claims, key) = fixture();
        let jwt = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test-only"),
        )
        .unwrap();
        assert!(verify_with_key(&jwt, "beacon-dev", &key).is_err());
        assert!(verify_with_key("eyJhbGciOiJub25lIn0.e30.", "beacon-dev", &key).is_err());
    }
}
