pub mod investment;
pub mod user;

use std::env;

use axum::{
    extract::{Path, Query, State},
    middleware,
    response::Redirect,
    routing::get,
    Extension, Json, Router,
};
use redis::{Commands, JsonCommands};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    db::item::{get_item_icon, get_price_object},
    error::{Error, Result},
    guard::guard,
    jwt::User,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/inventory", get(get_inventory))
        .route("/prices", get(get_prices))
        .route("/currencies", get(get_currencies))
        .route("/icon/:market_hash_name", get(get_icon))
        .nest("/investment", investment::routes())
        .nest("/user", user::routes())
        .route_layer(middleware::from_fn(guard))
}

#[derive(Serialize, Deserialize, Debug)]
struct Inventory {
    descriptions: Vec<Description>,
    total_inventory_count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Description {
    icon_url: String,
    name: String,
    market_hash_name: String,
    marketable: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct CustomInventory {
    items: Vec<Description>,
    total_inventory_count: i32,
}

async fn get_inventory(
    State(mut state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<CustomInventory>> {
    let proxy = reqwest::Proxy::all(env::var("PROXY_URL").unwrap())
        .map_err(|_| Error::ProxyCreationFail)?;

    let client = reqwest::Client::builder()
        .proxy(proxy)
        .build()
        .map_err(|_| Error::HttpClientCreationFail)?;

    let steam_id = user.steam_id()?;

    let steam_inventory_endpoint =
        format!("https://steamcommunity.com/inventory/{steam_id}/730/2?l=english&count=1000");

    let key = format!("inventory-{steam_id}");

    let cached_inventory: Option<String> = state
        .redis
        .json_get(&key, ".")
        .map_err(|_| Error::RedisGetFail)?;

    if cached_inventory.is_none() {
        let new_inventory: Inventory = client
            .get(steam_inventory_endpoint)
            .send()
            .await
            .map_err(|_| Error::InventoryFetchFail)?
            .json()
            .await
            .map_err(|_| Error::InventoryParseFail)?;

        let items = new_inventory
            .descriptions
            .into_iter()
            .filter(|desc| desc.marketable == 1)
            .collect();

        let custom_inventory = CustomInventory {
            items,
            total_inventory_count: new_inventory.total_inventory_count,
        };

        state
            .redis
            .json_set(
                &key,
                ".",
                &serde_json::to_string(&custom_inventory).map_err(|_| Error::InventoryParseFail)?,
            )
            .map_err(|_| Error::RedisSetFail)?;

        state
            .redis
            .expire(&key, 60 * 30)
            .map_err(|_| Error::RedisExpireFail)?;

        return Ok(Json(custom_inventory));
    }

    let cached_inventory = cached_inventory.unwrap().replace("\\", "");

    let inventory: CustomInventory =
        serde_json::from_str(&cached_inventory[1..cached_inventory.len() - 1])
            .map_err(|_| Error::InventoryParseFail)?;

    Ok(Json(inventory))
}

#[derive(Deserialize)]
struct ItemData {
    market_hash_name: String,
}

#[derive(Serialize, Deserialize)]
struct SteamPrices {
    last_24h: Option<f32>,
    last_7d: Option<f32>,
    last_30d: Option<f32>,
    last_90d: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct SkinportPrices {
    suggested_price: Option<f32>,
    starting_at: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct Price {
    price: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct BuffPrices {
    starting_at: Option<Price>,
    highest_order: Option<Price>,
}

#[derive(Serialize, Deserialize)]
struct Prices {
    steam: Option<SteamPrices>,
    skinport: Option<SkinportPrices>,
    buff163: Option<BuffPrices>,
}

async fn get_prices(
    Query(query): Query<ItemData>,
    State(mut state): State<AppState>,
) -> Result<Json<Prices>> {
    let prices = get_price_object(&mut state.redis, &query.market_hash_name)
        .await?
        .ok_or(Error::PricesFetchFail)?;

    let prices_value: Value = serde_json::from_str(&prices).map_err(|_| Error::PricesParseFail)?;

    let prices = prices_value.get(0).ok_or(Error::PricesFetchFail)?;

    let prices: Prices =
        serde_json::from_value(prices.clone()).map_err(|_| Error::PricesParseFail)?;

    Ok(Json(prices))
}

#[derive(Deserialize, Serialize)]
#[allow(non_snake_case)]
struct CurrencyRates {
    EUR: f32,
    CNY: f32,
}

async fn get_currencies(State(mut state): State<AppState>) -> Result<Json<CurrencyRates>> {
    let cached_rates: Option<String> = state
        .redis
        .json_get("currency_rates", ".")
        .map_err(|_| Error::RedisGetFail)?;

    if cached_rates.is_none() {
        let client = reqwest::Client::builder()
            .gzip(true)
            .build()
            .map_err(|_| Error::HttpClientCreationFail)?;

        let new_rates: CurrencyRates = client
            .get("https://prices.csgotrader.app/latest/exchange_rates.json")
            .send()
            .await
            .map_err(|_| Error::RatesFetchFail)?
            .json()
            .await
            .map_err(|_| Error::RatesParseFail)?;

        state
            .redis
            .json_set("currency_rates", ".", &new_rates)
            .map_err(|_| Error::RedisSetFail)?;

        state
            .redis
            .expire("currency_rates", 3600 * 3)
            .map_err(|_| Error::RedisExpireFail)?;

        return Ok(Json(new_rates));
    }

    let rates: CurrencyRates =
        serde_json::from_str(&cached_rates.unwrap()).map_err(|_| Error::RatesParseFail)?;

    Ok(Json(rates))
}

async fn get_icon(
    Path(market_hash_name): Path<String>,
    State(mut state): State<AppState>,
) -> Result<Redirect> {
    let icon_url = get_item_icon(&mut state.redis, &market_hash_name)
        .await?
        .ok_or(Error::ItemMissingImage)?;

    Ok(Redirect::to(&icon_url))
}
