use ntex::web;
use ntex::web::HttpResponse;
use thiserror::Error;

// 自定义错误类型
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum AppError {
    #[error("error: `{0}`")]
    Anyhow(#[from] anyhow::Error),
    #[error("not found")]
    NotFound,
    #[error("bad request")]
    BadRequest,
}

impl web::error::WebResponseError for AppError {}

pub type HandlerResponse = Result<HttpResponse, AppError>;