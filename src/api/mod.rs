pub mod investment;
pub mod user;

use std::env;

use axum::{
    extract::{Path, Query, State},
    middleware,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Extension, Json, Router,
};
use http::HeaderMap;
use redis::{Commands, JsonCommands};
use serde::{Deserialize, Serialize};

use crate::{
    db::item::{get_item_icon, get_item_prices},
    error::{Error, Result},
    guard::guard,
    jwt::User,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // .route("/inventory", get(get_inventory))
        .route("/inventory/price-check", post(price_check))
        // .route("/prices", get(get_prices))
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
    market_hash_name: String,
    marketable: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct CustomInventory {
    items: Vec<InventoryItem>,
    total_inventory_count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct InventoryItem {
    market_hash_name: String,
    prices: Prices,
    count: u32,
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
            .filter(|desc| desc.marketable == 1);

        let mut new_items = vec![];

        for item in items.into_iter() {
            new_items.push(InventoryItem {
                prices: get_item_prices(&mut state, &item.market_hash_name).await?,
                market_hash_name: item.market_hash_name,
                count: 1,
            });
        }

        let custom_inventory = CustomInventory {
            total_inventory_count: 0,
            items: new_items,
        };

        state
            .redis
            .json_set(&key, ".", &custom_inventory)
            .map_err(|_| Error::RedisSetFail)?;

        state
            .redis
            .expire(&key, 60 * 30)
            .map_err(|_| Error::RedisExpireFail)?;

        return Ok(Json(custom_inventory));
    }

    let inventory: CustomInventory =
        serde_json::from_str(&cached_inventory.unwrap()).map_err(|_| Error::InventoryParseFail)?;

    Ok(Json(inventory))
}

#[derive(Deserialize)]
struct ItemData {
    market_hash_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SteamPrices {
    pub last_24h: Option<f32>,
    pub last_7d: Option<f32>,
    pub last_30d: Option<f32>,
    pub last_90d: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SkinportPrices {
    pub suggested_price: Option<f32>,
    pub starting_at: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Price {
    pub price: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuffPrices {
    pub starting_at: Option<Price>,
    pub highest_order: Option<Price>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Prices {
    pub steam: Option<SteamPrices>,
    pub skinport: Option<SkinportPrices>,
    pub buff163: Option<BuffPrices>,
}

async fn get_prices(
    Query(query): Query<ItemData>,
    State(mut state): State<AppState>,
) -> Result<Json<Prices>> {
    Ok(Json(
        get_item_prices(&mut state, &query.market_hash_name).await?,
    ))
}

#[derive(Deserialize, Serialize)]
#[allow(non_snake_case)]
struct CurrencyRates {
    EUR: f32,
    CNY: f32,
    TRY: f32,
    PLN: f32,
    GBP: f32,
    UAH: f32,
    KRW: f32,
    BRL: f32,
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

#[derive(Deserialize, Debug)]
struct Item {
    #[serde(rename = "markethashname")]
    market_hash_name: String,
    count: u32,
}

#[derive(Deserialize, Debug)]
struct InventoryItems {
    items: Vec<Item>,
}

async fn price_check(
    State(mut state): State<AppState>,
    Json(body): Json<InventoryItems>,
) -> Result<impl IntoResponse> {
    let mut new_items = vec![];

    for item in body.items.into_iter() {
        new_items.push(InventoryItem {
            prices: get_item_prices(&mut state, &item.market_hash_name).await?,
            market_hash_name: item.market_hash_name,
            count: item.count,
        });
    }

    let custom_inventory = CustomInventory {
        total_inventory_count: new_items.iter().map(|i| i.count).sum(),
        items: new_items,
    };

    let mut headers = HeaderMap::new();

    headers.insert("Cache-Control", "public, max-age=1800".parse().unwrap());

    Ok((headers, Json(custom_inventory)).into_response())
}
