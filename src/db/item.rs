use sqlx::PgPool;

use crate::error::{Error, Result};

pub async fn item_exists(pool: &PgPool, market_hash_name: &str) -> Result<bool> {
    let sql = r"
        select * from items
        where market_hash_name like $1
    ";

    let query = sqlx::query(sql).bind(market_hash_name);

    let item = query.fetch_optional(pool).await.map_err(|e| {
        println!("{e}");
        Error::PgFetchFail
    })?;

    match item {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

pub async fn create_item(pool: &PgPool, market_hash_name: &str, icon_url: &str) -> Result<()> {
    let sql = r"
        insert into items
        (market_hash_name, icon_url) values ($1, $2)
    ";

    sqlx::query(sql)
        .bind(market_hash_name)
        .bind(icon_url)
        .execute(pool)
        .await
        .map_err(|e| {
            println!("{e}");
            Error::PgInsertFail
        })?;

    Ok(())
}
