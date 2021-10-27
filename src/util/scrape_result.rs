use reqwest::{Error, Response};
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::serde::Serialize;

pub type ScrapeResult<T> = Result<T, ScrapeError>;

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum CaptchaPayload {
    Captcha { url: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub enum ErrorPayload {
    Error { reason: String },
}

#[derive(Debug, Responder)]
pub enum ScrapeError {
    #[response(status = 200, content_type = "json")]
    Captcha(Json<CaptchaPayload>),
    #[response(status = 500)]
    Error(Json<ErrorPayload>),
}

pub async fn scrape_result(
    response: Result<Response, Error>,
    error: &str,
) -> ScrapeResult<Response> {
    match response {
        Ok(r) => {
            let url = r.url().to_string();
            if url.contains("validate.perfdrive.com") {
                Err(ScrapeError::Captcha(Json(CaptchaPayload::Captcha { url })))
            } else {
                Ok(r)
            }
        }
        Err(e) => Err(ScrapeError::Error(Json(ErrorPayload::Error {
            reason: if let Some(status) = e.status() {
                format!("{}:\n  status: {}\n  error: {}", error, status.as_u16(), e)
            } else {
                format!("{}:\n  error: {}", error, e)
            },
        }))),
    }
}
