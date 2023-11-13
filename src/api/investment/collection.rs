use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::collection::{create_collection, drop_collection, get_collections, Collection},
    error::{Error, Result},
    jwt::User,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/create", post(new_collection))
        .route("/all", get(all_collections))
        .route("/:coll_id", delete(delete_collection))
}

#[derive(Deserialize)]
struct CollectionReq {
    name: String,
}

async fn new_collection(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(body): Json<CollectionReq>,
) -> Result<Json<Collection>> {
    Ok(Json(
        create_collection(
            &state.pg,
            &user.steam.id.ok_or(Error::SteamMissingId)?,
            &body.name,
        )
        .await?,
    ))
}

#[derive(Serialize)]
struct Collections {
    collections: Vec<Collection>,
}

async fn all_collections(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Collections>> {
    let collections =
        get_collections(&state.pg, user.steam.id.ok_or(Error::SteamMissingId)?).await?;

    Ok(Json(Collections { collections }))
}

async fn delete_collection(
    Path(col_id): Path<i32>,
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<()> {
    drop_collection(
        &state.pg,
        user.steam.id.ok_or(Error::SteamMissingId)?,
        col_id,
    )
    .await?;

    Ok(())
}
