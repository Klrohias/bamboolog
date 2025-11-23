use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub struct ApiResponse<T = ()> {
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn new(code: StatusCode, message: String, data: T) -> Self {
        Self {
            code: code.as_u16() as i32,
            message: Some(message),
            data: Some(data),
        }
    }

    pub fn new_any(code: i32, message: String, data: T) -> Self {
        Self {
            code,
            message: Some(message),
            data: Some(data),
        }
    }

    pub fn ok(data: T) -> Self {
        Self {
            code: 200,
            message: None,
            data: Some(data),
        }
    }
}

impl ApiResponse<()> {
    pub fn unauthorized() -> Self {
        Self {
            code: StatusCode::UNAUTHORIZED.as_u16() as i32,
            message: Some("unauthorized".to_string()),
            data: None,
        }
    }

    pub fn internal_server_error() -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            message: Some("internal server error".to_string()),
            data: None,
        }
    }

    pub fn code(code: StatusCode) -> Self {
        Self {
            code: code.as_u16() as i32,
            message: Some(code.as_str().to_string()),
            data: None,
        }
    }

    pub fn code_and_message(code: StatusCode, message: impl AsRef<str>) -> Self {
        Self {
            code: code.as_u16() as i32,
            message: Some(message.as_ref().to_owned()),
            data: None,
        }
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        if self.code >= 100 && self.code < 1000 {
            (
                StatusCode::from_u16(self.code as u16).unwrap(),
                Json(serde_json::json!({
                    "code": self.code,
                    "message": self.message,
                    "data":self.data,
                })),
            )
                .into_response()
        } else {
            Json(serde_json::json!({
                "code": self.code,
                "message": self.message,
                "data":self.data,
            }))
            .into_response()
        }
    }
}
