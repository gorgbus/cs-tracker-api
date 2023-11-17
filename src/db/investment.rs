use serde::{Deserialize, Serialize};
use sqlx::{types::Decimal, FromRow, PgPool, Type};

use crate::{
    api::investment::EditInvestmentReq,
    error::{Error, Result},
};

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
struct Investment {
    inv_id: i32,
    steam_id: String,
    item: String,
    collection: i32,
    cost: Decimal,
    amount: i32,
    currency: Currencies,
}

#[derive(Debug, Type, Serialize, Deserialize, Clone)]
#[sqlx(type_name = "currencies")]
#[allow(non_camel_case_types)]
pub enum Currencies {
    USD,
    EUR,
    CNY,
}

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct CustomInvestment {
    inv_id: i32,
    steam_id: String,
    item: String,
    collection: i32,
    col_name: String,
    cost: Decimal,
    amount: i32,
    currency: Currencies,
}

pub async fn create_investment(
    pool: &PgPool,
    steam_id: String,
    market_hash_name: &str,
    col_id: i32,
    cost: f32,
    amount: i32,
    currency: Currencies,
) -> Result<CustomInvestment> {
    let sql = r"
        insert into investments
        (steam_id, item, collection, cost, amount, currency) values ($1, $2, $3, $4, $5, $6)
        returning *
    ";

    let investment: Investment = sqlx::query_as(sql)
        .bind(steam_id)
        .bind(market_hash_name)
        .bind(col_id)
        .bind(cost)
        .bind(amount)
        .bind(currency)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            println!("{e}");
            Error::PgInsertFail
        })?;

    get_investment(pool, investment.inv_id).await
}

pub async fn get_investment(pool: &PgPool, inv_id: i32) -> Result<CustomInvestment> {
    let sql = r"
        select inv.*, c.name as col_name
        from investments inv inner join collections c on c.col_id = inv.collection
        where inv.inv_id = $1
    ";

    let mut invest: CustomInvestment = sqlx::query_as(sql)
        .bind(inv_id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            println!("{e}");
            Error::PgFetchFail
        })?;

    invest.cost.rescale(2);

    Ok(invest)
}

pub async fn get_investments(pool: &PgPool, steam_id: String) -> Result<Vec<CustomInvestment>> {
    let sql = r"
        select inv.*, c.name as col_name
        from investments inv inner join collections c on c.col_id = inv.collection
        where inv.steam_id = $1
        order by inv.inv_id asc
    ";

    let invests: Vec<CustomInvestment> = sqlx::query_as(sql)
        .bind(steam_id)
        .fetch_all(pool)
        .await
        .map_err(|_| Error::PgFetchFail)?;

    let invests = invests
        .into_iter()
        .map(|mut i| {
            i.cost.rescale(2);
            i
        })
        .collect::<Vec<CustomInvestment>>();

    Ok(invests)
}

pub async fn get_investments_by_coll(
    pool: &PgPool,
    steam_id: String,
    col_id: i32,
) -> Result<Vec<CustomInvestment>> {
    let sql = r"
        select inv.*, c.name as col_name
        from investments inv inner join collections c on c.col_id = inv.collection
        where inv.steam_id = $1 and c.col_id = $2
        order by inv.inv_id asc
    ";

    let invests: Vec<CustomInvestment> = sqlx::query_as(sql)
        .bind(steam_id)
        .bind(col_id)
        .fetch_all(pool)
        .await
        .map_err(|_| Error::PgFetchFail)?;

    let invests = invests
        .into_iter()
        .map(|mut i| {
            i.cost.rescale(2);
            i
        })
        .collect::<Vec<CustomInvestment>>();

    Ok(invests)
}

pub async fn drop_investment(pool: &PgPool, steam_id: String, inv_id: i32) -> Result<()> {
    let sql = r"
        delete from investments
        where steam_id = $1 and inv_id = $2
    ";

    sqlx::query(sql)
        .bind(steam_id)
        .bind(inv_id)
        .execute(pool)
        .await
        .map_err(|_| Error::PgDeleteFail)?;

    Ok(())
}

pub async fn update_investment(
    pool: &PgPool,
    steam_id: String,
    inv_id: i32,
    data: EditInvestmentReq,
) -> Result<CustomInvestment> {
    let sql = r"
        update investments
        set collection = $1, amount = $2, cost = $3, currency = $4
        where steam_id = $5 and inv_id = $6
    ";

    sqlx::query(sql)
        .bind(data.col_id)
        .bind(data.amount)
        .bind(data.cost)
        .bind(data.currency)
        .bind(steam_id)
        .bind(inv_id)
        .execute(pool)
        .await
        .map_err(|_| Error::PgUpdateFail)?;

    get_investment(pool, inv_id).await
}
