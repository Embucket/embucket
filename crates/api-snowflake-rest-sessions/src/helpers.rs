use chrono::offset::Local;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use time::Duration;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub struct Claims {
    pub sub: String, // token issued to a particular user
    pub aud: String, // validate audience since as it can be deployed on multiple hosts
    pub iat: i64,    // Issued At
    pub exp: i64,    // Expiration Time
    pub session_id: String,
}

#[must_use]
pub fn jwt_claims(username: &str, audience: &str, expiration: Duration) -> Claims {
    let now = Local::now();
    let iat = now.timestamp();
    let exp = now.timestamp() + expiration.whole_seconds();

    Claims {
        sub: username.to_string(),
        aud: audience.to_string(),
        iat,
        exp,
        session_id: Uuid::new_v4().to_string(),
    }
}

pub fn get_claims_validate_jwt_token(
    token: &str,
    audience: &str,
    jwt_secret: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.leeway = 5;
    validation.set_audience(&[audience]);
    validation.set_required_spec_claims(&["exp", "aud"]);

    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());

    let decoded = decode::<Claims>(token, &decoding_key, &validation)?;

    Ok(decoded.claims)
}

pub fn create_jwt<T>(claims: &T, jwt_secret: &str) -> Result<String, jsonwebtoken::errors::Error>
where
    T: Serialize,
{
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
}

#[must_use]
pub fn ensure_jwt_secret_is_valid(jwt_secret: &str) -> Option<String> {
    if jwt_secret.is_empty() {
        return None;
    }
    Some(jwt_secret.to_string())
}
