use axum::{middleware::Next, response::Response};
use http::Request;
use tower_cookies::Cookies;

use crate::{
    error::{Error, Result},
    jwt::verify_token,
};

pub async fn guard<T>(cookies: Cookies, mut req: Request<T>, next: Next<T>) -> Result<Response> {
    let token = &cookies.get("access").ok_or(Error::AuthMissingCookie)?;
    let token = token.value();

    let user = verify_token(token)?;

    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}
