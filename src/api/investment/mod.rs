use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        investment::{
            create_investment, drop_investment, get_investments, get_investments_by_coll,
            update_investment, Currencies, CustomInvestment,
        },
        item::item_exists,
    },
    error::{Error, Result},
    jwt::User,
    state::AppState,
};

pub mod collection;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/create", post(new_investment))
        .route("/all", get(all_investments))
        .route("/:inv_id", delete(delete_investment))
        .route("/:inv_id", post(edit_investment))
        .nest("/collection", collection::routes())
}

#[derive(Deserialize)]
struct InvestmentReq {
    market_hash_name: String,
    col_id: i32,
    cost: f32,
    amount: i32,
    currency: Currencies,
}

async fn new_investment(
    State(mut state): State<AppState>,
    Extension(user): Extension<User>,
    Json(body): Json<InvestmentReq>,
) -> Result<Json<CustomInvestment>> {
    match item_exists(&mut state.redis, &body.market_hash_name).await? {
        None => return Err(Error::InvalidHashName),
        _ => (),
    }

    Ok(Json(
        create_investment(
            &state.pg,
            user.steam_id()?,
            &body.market_hash_name,
            body.col_id,
            body.cost,
            body.amount,
            body.currency,
        )
        .await?,
    ))
}

#[derive(Serialize)]
struct Investments {
    investments: Vec<CustomInvestment>,
}

#[derive(Deserialize)]
struct InvestmentQuery {
    col_id: Option<i32>,
}

async fn all_investments(
    Query(query): Query<InvestmentQuery>,
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Investments>> {
    let investments = match query.col_id {
        Some(col_id) => get_investments_by_coll(&state.pg, user.steam_id()?, col_id).await?,
        _ => get_investments(&state.pg, user.steam_id()?).await?,
    };

    Ok(Json(Investments { investments }))
}

async fn delete_investment(
    Path(inv_id): Path<i32>,
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<()> {
    drop_investment(&state.pg, user.steam_id()?, inv_id).await
}

#[derive(Deserialize)]
pub struct EditInvestmentReq {
    pub col_id: i32,
    pub amount: i32,
    pub cost: f32,
    pub currency: Currencies,
}

async fn edit_investment(
    Path(inv_id): Path<i32>,
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(body): Json<EditInvestmentReq>,
) -> Result<Json<CustomInvestment>> {
    Ok(Json(
        update_investment(&state.pg, user.steam_id()?, inv_id, body).await?,
    ))
}
