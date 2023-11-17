use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde::Serialize;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ProxyCreationFail,
    HttpClientCreationFail,
    UriCreateFail,

    InventoryFetchFail,
    InventoryParseFail,

    PricesFetchFail,
    PricesParseFail,

    RatesFetchFail,
    RatesParseFail,

    ItemsFetchFail,
    ItemsParseFail,
    ItemMissingImage,
    InvalidHashName,

    SteamMissingId,
    SteamMissingAsset,
    SteamMissingDesc,

    RedisGetFail,
    RedisSetFail,
    RedisExpireFail,

    PgFetchFail,
    PgInsertFail,
    PgDeleteFail,
    PgUpdateFail,

    JwtInvalidToken,

    AuthMissingCookie,
}
#[derive(Serialize)]
#[allow(non_camel_case_types)]
pub enum ClientError {
    NO_AUTH,
    SERVICE_ERROR,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();

        response.extensions_mut().insert(self);

        response
    }
}

impl Error {
    pub fn client_status_and_error(&self) -> (StatusCode, ClientError) {
        match self {
            Self::JwtInvalidToken => (StatusCode::UNAUTHORIZED, ClientError::NO_AUTH),

            Self::AuthMissingCookie => (StatusCode::FORBIDDEN, ClientError::NO_AUTH),

            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::SERVICE_ERROR,
            ),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
