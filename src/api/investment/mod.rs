use std::env;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    db::{
        investment::{
            create_investment, drop_investment, get_investments, get_investments_by_coll,
            Currencies, CustomInvestment,
        },
        item::{create_item, item_exists},
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
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(body): Json<InvestmentReq>,
) -> Result<Json<CustomInvestment>> {
    if !item_exists(&state.pg, &body.market_hash_name).await? {
        let proxy = reqwest::Proxy::all(env::var("PROXY_URL").unwrap())
            .map_err(|_| Error::ProxyCreationFail)?;

        let client = reqwest::Client::builder()
            .proxy(proxy)
            .build()
            .map_err(|_| Error::HttpClientCreationFail)?;

        let steam_market_url = format!(
            "https://steamcommunity.com/market/listings/730/{}/render?currency=2&format=json",
            body.market_hash_name
        );

        let item_info: Value = client
            .get(steam_market_url)
            .send()
            .await
            .map_err(|_| Error::PricesFetchFail)?
            .json()
            .await
            .map_err(|_| Error::PricesParseFail)?;

        let assets = item_info
            .get("assets")
            .ok_or(Error::SteamMissingAsset)?
            .get("730")
            .ok_or(Error::SteamMissingAsset)?
            .get("2")
            .ok_or(Error::SteamMissingAsset)?
            .as_object()
            .ok_or(Error::SteamMissingAsset)?;

        let mut icon_url = None;

        for (_, ctx) in assets.iter() {
            icon_url = ctx
                .get("icon_url")
                .ok_or(Error::SteamMissingAsset)?
                .as_str();
        }

        let icon_url = icon_url.ok_or(Error::SteamMissingAsset)?;

        create_item(&state.pg, &body.market_hash_name, icon_url).await?;
    }

    Ok(Json(
        create_investment(
            &state.pg,
            user.steam.id.ok_or(Error::SteamMissingId)?,
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
        Some(col_id) => {
            get_investments_by_coll(
                &state.pg,
                user.steam.id.ok_or(Error::SteamMissingId)?,
                col_id,
            )
            .await?
        }
        _ => get_investments(&state.pg, user.steam.id.ok_or(Error::SteamMissingId)?).await?,
    };

    Ok(Json(Investments { investments }))
}

async fn delete_investment(
    Path(inv_id): Path<i32>,
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<()> {
    drop_investment(
        &state.pg,
        user.steam.id.ok_or(Error::SteamMissingId)?,
        inv_id,
    )
    .await?;

    Ok(())
}
