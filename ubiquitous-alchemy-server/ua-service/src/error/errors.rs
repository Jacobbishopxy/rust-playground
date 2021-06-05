//!

use actix_web::{dev, error::ResponseError, BaseHttpResponse};
use derive_more::Display;

use ua_dao::error::DaoError;

#[derive(Debug, Display)]
pub enum ServiceError {
    #[display(fmt = "Dao error {}", _0)]
    DaoError(DaoError),

    #[display(fmt = "Internal server error")]
    InternalServerError,

    #[display(fmt = "Bad Request: {}", _0)]
    BadRequest(String),
}

impl From<DaoError> for ServiceError {
    fn from(error: DaoError) -> Self {
        match error {
            e @ DaoError::DatabaseGeneralError(_) => ServiceError::DaoError(e),
            e @ DaoError::DatabaseConnectionError(_) => ServiceError::DaoError(e),
            e @ DaoError::DatabaseOperationError(_) => ServiceError::DaoError(e),
        }
    }
}

// todo: redo after upgrade actix_web
impl ResponseError for ServiceError {
    fn error_response(&self) -> BaseHttpResponse<actix_web::dev::Body> {
        match self {
            ServiceError::DaoError(e) => {
                let e_s = serde_json::to_string(e).unwrap();
                BaseHttpResponse::internal_server_error().set_body(dev::Body::from_message(e_s))
            }
            ServiceError::InternalServerError => BaseHttpResponse::internal_server_error()
                .set_body(dev::Body::from_message("Internal Server Error")),
            ServiceError::BadRequest(e) => {
                BaseHttpResponse::bad_request().set_body(dev::Body::from_message(e.to_owned()))
            }
        }
    }
}