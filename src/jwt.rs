use std::env;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub fn verify_token(token: &str) -> Result<User> {
    let decoding_key =
        DecodingKey::from_rsa_pem(env::var("TOKEN_PUBLIC_KEY").unwrap().as_bytes()).unwrap();

    let token_data = decode::<Claims>(token, &decoding_key, &Validation::new(Algorithm::RS256))
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
    pub steam: Account,
}

impl User {
    pub fn steam_id(self) -> Result<String> {
        self.steam.id.ok_or(Error::SteamMissingId)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: Option<String>,
}
