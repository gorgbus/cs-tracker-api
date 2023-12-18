use axum::{extract::State, routing::get, Extension, Router};

use crate::{
    db::{
        collection::create_collection,
        user::{create_user, user_exists},
    },
    error::{Error, Result},
    jwt::User,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(get_user))
}

async fn get_user(State(state): State<AppState>, Extension(user): Extension<User>) -> Result<()> {
    let steam_id = user.steam.id.ok_or(Error::SteamMissingId)?;

    if !user_exists(&state.pg, &steam_id).await? {
        create_user(&state.pg, &steam_id).await?;

        create_collection(&state.pg, &steam_id, "Collection 1").await?;
    }

    Ok(())
}
