use std::fmt::Display;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::utils::ApiResponse;

pub trait FailibleOperationExt<T, E> {
    fn traced(self, op: impl FnOnce(&E) -> ()) -> Result<T, E>;
}

impl<T, E> FailibleOperationExt<T, E> for Result<T, E>
where
    E: Display,
{
    fn traced(self, op: impl FnOnce(&E) -> ()) -> Result<T, E> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                op(&e);
                Err(e)
            }
        }
    }
}

pub trait HttpFailibleOperationExt<T, E> {
    fn response(self) -> Result<T, Response>;
    fn traced_and_response(self, op: impl FnOnce(&E) -> ()) -> Result<T, Response>;
}

impl<T, E> HttpFailibleOperationExt<T, E> for Result<T, E>
where
    E: Display,
{
    fn response(self) -> Result<T, Response> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiResponse::internal_server_error(),
            )
                .into_response()),
        }
    }

    fn traced_and_response(self, op: impl FnOnce(&E) -> ()) -> Result<T, Response> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                op(&e);

                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiResponse::internal_server_error(),
                )
                    .into_response())
            }
        }
    }
}
