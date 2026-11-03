use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_cookies::Cookies;

use crate::http::AuthState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthSource {
    Cookie,
    Bearer,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub email: String,
    pub auth_source: AuthSource,
}

pub fn create_jwt(
    user_id: &str,
    email: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = (now + Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_state = AuthState::from_ref(state);

        // 1. Cek Header Authorization Bearer terlebih dahulu (Mobile Client / API)
        let token_from_bearer = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .and_then(|val| {
                if val.starts_with("Bearer ") {
                    Some((val[7..].trim().to_string(), AuthSource::Bearer))
                } else {
                    None
                }
            });

        // 2. Jika tidak ada di header, cek Cookie (Web Client / SPA)
        let token_pair = match token_from_bearer {
            Some(pair) => Some(pair),
            None => Cookies::from_request_parts(parts, state)
                .await
                .ok()
                .and_then(|cookies| cookies.get("auth_token").map(|c| c.value().to_string()))
                .map(|token| (token, AuthSource::Cookie)),
        };

        let (token, auth_source) = match token_pair {
            Some(pair) => pair,
            None => {
                let err_body = Json(json!({
                    "status": "error",
                    "message": "Unauthenticated: Missing cookie or Bearer token",
                    "data": null
                }));
                return Err((StatusCode::UNAUTHORIZED, err_body).into_response());
            }
        };

        // 3. Cek apakah token telah di-blacklist (misalnya sudah di-logout via Redis)
        if auth_state
            .blacklist
            .is_blacklisted(&token)
            .await
            .unwrap_or(false)
        {
            let err_body = Json(json!({
                "status": "error",
                "message": "Token has been revoked or logged out",
                "data": null
            }));
            return Err((StatusCode::UNAUTHORIZED, err_body).into_response());
        }

        // 4. Validasi signature & masa aktif token JWT
        let claims = match decode_jwt(&token, &auth_state.jwt_secret) {
            Ok(c) => c,
            Err(_) => {
                let err_body = Json(json!({
                    "status": "error",
                    "message": "Invalid or expired token",
                    "data": null
                }));
                return Err((StatusCode::UNAUTHORIZED, err_body).into_response());
            }
        };

        Ok(AuthUser {
            user_id: claims.sub,
            email: claims.email,
            auth_source,
        })
    }
}

