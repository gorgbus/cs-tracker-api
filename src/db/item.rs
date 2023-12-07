use redis::{Commands, JsonCommands};
use serde::Serialize;
use serde_json::Value;
use sqlx::{prelude::FromRow, Postgres, QueryBuilder};

use crate::{
    error::{Error, Result},
    state::AppState,
};

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

pub async fn item_exists(state: &mut AppState, market_hash_name: &str) -> Result<Option<()>> {
    match get_price_object(state, market_hash_name).await? {
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
    state: &mut AppState,
    market_hash_name: &str,
) -> Result<Option<String>> {
    let cached_prices: Option<String> = state
        .redis
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

        state
            .redis
            .json_set("csgotrader_prices", ".", &new_prices)
            .map_err(|_| Error::RedisSetFail)?;

        state
            .redis
            .expire("csgotrader_prices", 3600 * 8)
            .map_err(|_| Error::RedisExpireFail)?;

        let sql = r"
            delete from items
        ";

        sqlx::query(sql)
            .execute(&state.pg)
            .await
            .map_err(|_| Error::PgDeleteFail)?;

        if let Some(prices_object) = new_prices.as_object() {
            let mut query_builder: QueryBuilder<Postgres> =
                QueryBuilder::new("insert into items(market_hash_name) ");

            let keys = prices_object.keys();

            query_builder.push_values(keys, |mut b, key| {
                b.push_bind(key);
            });

            let query = query_builder.build();

            query
                .execute(&state.pg)
                .await
                .map_err(|_| Error::PgInsertFail)?;
        }
    }

    state
        .redis
        .json_get("csgotrader_prices", format!("$[\"{}\"]", market_hash_name))
        .map_err(|_| Error::RedisGetFail)
}

#[derive(FromRow, Debug, Serialize)]
pub struct Item {
    pub market_hash_name: String,
}

pub async fn suggest_items(sqlite: &sqlx::PgPool, item_name: String) -> Result<Vec<Item>> {
    // let sql = r"
    //     select * from items
    //     where market_hash_name % $1 and to_tsvector('english', market_hash_name) @@ to_tsquery('english', $1)
    //     order by market_hash_name
    //     limit 5
    // ";

    let sql = r"
        with search as (
            select to_tsquery(string_agg(lexeme || ':*', ' & ' order by positions)) as query
            from unnest(to_tsvector($1))
        )
        select items.*
        from items, search
        where (items.market_hash_name @@ search.query)
        limit 5
    ";

    sqlx::query_as(sql)
        .bind(item_name)
        .fetch_all(sqlite)
        .await
        .map_err(|e| {
            println!("{e:?}");
            Error::PgFetchFail
        })
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
