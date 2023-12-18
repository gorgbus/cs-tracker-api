use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::error::{Error, Result};

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct Collection {
    col_id: i32,
    steam_id: String,
    name: String,
}

pub async fn create_collection(pool: &PgPool, steam_id: &str, name: &str) -> Result<Collection> {
    let sql = r"
        insert into collections
        (steam_id, name) values ($1, $2)
        returning *
    ";

    sqlx::query_as(sql)
        .bind(steam_id)
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            println!("{e}");
            Error::PgInsertFail
        })
}

pub async fn get_collections(pool: &PgPool, steam_id: String) -> Result<Vec<Collection>> {
    let sql = r"
        select * from collections
        where steam_id = $1
        order by col_id asc
    ";

    sqlx::query_as(sql)
        .bind(steam_id)
        .fetch_all(pool)
        .await
        .map_err(|_| Error::PgFetchFail)
}

pub async fn drop_collection(pool: &PgPool, steam_id: String, col_id: i32) -> Result<()> {
    let sql = r"
        delete from collections
        where steam_id = $1 and col_id = $2
    ";

    sqlx::query(sql)
        .bind(steam_id)
        .bind(col_id)
        .execute(pool)
        .await
        .map_err(|_| Error::PgDeleteFail)?;

    Ok(())
}

pub async fn update_collection(
    pool: &PgPool,
    steam_id: String,
    col_id: i32,
    name: String,
) -> Result<Collection> {
    let sql = r"
        update collections
        set name = $1
        where steam_id = $2 and col_id = $3
        returning *
    ";

    sqlx::query_as(sql)
        .bind(name)
        .bind(steam_id)
        .bind(col_id)
        .fetch_one(pool)
        .await
        .map_err(|_| Error::PgUpdateFail)
}
