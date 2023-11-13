use std::env;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const KEY: Lazy<DecodingKey> =
    Lazy::new(|| DecodingKey::from_secret(env::var("JWT_ACCESS_KEY").unwrap().as_bytes()));

pub fn verify_token(token: &str) -> Result<User> {
    let token_data = decode::<Claims>(token, &KEY, &Validation::new(Algorithm::HS256))
        .map_err(|_| Error::JwtInvalidToken)?;

    Ok(token_data.claims.user)
}

#[derive(Serialize, Deserialize)]
struct Claims {
    user: User,
    app_id: String,
    exp: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub user_id: i32,
    pub discord: Account,
    pub steam: Account,
    pub admin: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: Option<String>,
    pub avatar: Option<String>,
    pub username: Option<String>,
}
