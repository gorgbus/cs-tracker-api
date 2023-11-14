use redis::{Commands, JsonCommands};
use serde_json::Value;

use crate::error::{Error, Result};

pub async fn get_item_icon(
    redis: &mut redis::Client,
    market_hash_name: &str,
) -> Result<Option<String>> {
    let item_types = ["skins", "stickers", "crates", "agents", "patches"];

    for item_type in item_types.iter() {
        match get_icon(redis, item_type, market_hash_name).await? {
            Some(icon) => return Ok(Some(icon)),
            _ => (),
        }
    }

    Ok(None)
}

async fn get_icon(
    redis: &mut redis::Client,
    item_type: &str,
    market_hash_name: &str,
) -> Result<Option<String>> {
    let item = get_item_object(redis, item_type, market_hash_name).await?;

    match item {
        Some(json) => {
            if json == "[]" {
                return Ok(None);
            }

            let item: Value = serde_json::from_str(&json).map_err(|_| Error::ItemsParseFail)?;

            Ok(Some(
                item.get(0)
                    .ok_or(Error::ItemMissingImage)?
                    .get("image")
                    .ok_or(Error::ItemMissingImage)?
                    .as_str()
                    .ok_or(Error::ItemMissingImage)?
                    .to_string(),
            ))
        }
        _ => Ok(None),
    }
}

pub async fn item_exists(redis: &mut redis::Client, market_hash_name: &str) -> Result<Option<()>> {
    match get_price_object(redis, market_hash_name).await? {
        Some(json) => {
            if json == "[]" {
                return Ok(None);
            }

            Ok(Some(()))
        }
        _ => Ok(None),
    }
}

pub async fn get_price_object(
    redis: &mut redis::Client,
    market_hash_name: &str,
) -> Result<Option<String>> {
    let cached_prices: Option<String> = redis
        .json_get("csgotrader_prices", format!("$[\"{}\"]", market_hash_name))
        .map_err(|_| Error::RedisGetFail)?;

    if cached_prices.is_none() {
        let client = reqwest::Client::builder()
            .gzip(true)
            .build()
            .map_err(|_| Error::HttpClientCreationFail)?;

        let new_prices: Value = client
            .get("https://prices.csgotrader.app/latest/prices_v6.json")
            .send()
            .await
            .map_err(|_| Error::PricesFetchFail)?
            .json()
            .await
            .map_err(|_| Error::PricesParseFail)?;

        redis
            .json_set("csgotrader_prices", ".", &new_prices)
            .map_err(|_| Error::RedisSetFail)?;

        redis
            .expire("csgotrader_prices", 3600 * 8)
            .map_err(|_| Error::RedisExpireFail)?;
    }

    redis
        .json_get("csgotrader_prices", format!("$[\"{}\"]", market_hash_name))
        .map_err(|_| Error::RedisGetFail)
}

async fn get_item_object(
    redis: &mut redis::Client,
    item_type: &str,
    market_hash_name: &str,
) -> Result<Option<String>> {
    let key = format!("cs_{item_type}");
    let mut path = format!("$.[?@.name=='{market_hash_name}']");

    if item_type == "skins" {
        match market_hash_name.rfind("(") {
            Some(pos) => {
                path = format!(
                    "$.[?@.name=='{}']",
                    &market_hash_name[..pos - 1].replace("StatTrak™ ", "")
                );
            }
            _ => (),
        }
    }

    let items: Option<String> = redis.json_get(&key, ".").map_err(|_| Error::RedisGetFail)?;

    if items.is_none() {
        set_items(redis, item_type).await?;
    }

    redis.json_get(&key, &path).map_err(|_| Error::RedisGetFail)
}

async fn set_items(redis: &mut redis::Client, item_type: &str) -> Result<()> {
    let new_skins = get_items(item_type).await?;

    let key = format!("cs_{item_type}");

    redis
        .json_set(&key, ".", &new_skins)
        .map_err(|_| Error::RedisSetFail)?;

    redis
        .expire(&key, 3600 * 24)
        .map_err(|_| Error::RedisExpireFail)?;

    Ok(())
}

async fn get_items(item_type: &str) -> Result<Value> {
    let client = reqwest::Client::builder()
        .gzip(true)
        .build()
        .map_err(|_| Error::HttpClientCreationFail)?;

    let url = "https://bymykel.github.io/CSGO-API/api/en/";

    let new_items: Value = client
        .get(format!("{url}/{item_type}.json"))
        .send()
        .await
        .map_err(|_| Error::ItemsFetchFail)?
        .json()
        .await
        .map_err(|_| Error::ItemsParseFail)?;

    Ok(new_items)
}
