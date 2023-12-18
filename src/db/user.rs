use sqlx::PgPool;

use crate::error::{Error, Result};

pub async fn user_exists(pool: &PgPool, steam_id: &str) -> Result<bool> {
    let sql = r"
        select * from users
        where steam_id like $1
    ";

    let query = sqlx::query(sql).bind(steam_id);

    let user = query
        .fetch_optional(pool)
        .await
        .map_err(|_| Error::PgFetchFail)?;

    match user {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

pub async fn create_user(pool: &PgPool, steam_id: &str) -> Result<()> {
    let sql = r"
        insert into users
        (steam_id) values ($1)
    ";

    sqlx::query(sql)
        .bind(steam_id)
        .execute(pool)
        .await
        .map_err(|e| {
            println!("{e}");
            Error::PgInsertFail
        })?;

    Ok(())
}
