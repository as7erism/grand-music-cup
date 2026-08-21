use std::iter::{Empty, empty};

use axum::{Json, response::{Html, IntoResponse}};
use http::{HeaderName, HeaderValue, StatusCode};
use maud::Markup;
use serde::{Deserialize, Serialize};
use strum::EnumString;

use crate::api::{ApiError, ErrorAsJson};

#[derive(Clone, Copy, Debug, EnumString, Deserialize)]
enum UnionResponseKind {
    #[serde(rename = "json")]
    #[strum(serialize = "json")]
    Json,
    #[serde(rename = "html")]
    #[strum(serialize = "html")]
    Html,
}

enum UnionResponse<J, H, I = Empty<(HeaderName, HeaderValue)>>
where
    J: Serialize,
    H: IntoResponse,
    I: IntoIterator<Item = (HeaderName, HeaderValue)>
{
    Json((StatusCode, J, I)),
    Html((StatusCode, H, I)),
}

trait IntoUnionResponse<J, H, I = Empty<(HeaderName, HeaderValue)>>
where
    J: Serialize,
    H: IntoResponse,
    I: IntoIterator<Item = (HeaderName, HeaderValue)>,
{
    fn into_json(self) -> (StatusCode, J, I);
    fn into_html(self) -> (StatusCode, H, I);

    fn into_api_success(self, response_kind: UnionResponseKind) -> UnionResponse<J, H, I>
    where
        Self: Sized,
    {
        match response_kind {
            UnionResponseKind::Json => {
                let (status, json, headers) = self.into_json();
                UnionResponse::Json((status, json, headers))
            }
            UnionResponseKind::Html => {
                let (status, html, headers) = self.into_html();
                UnionResponse::Html((status, html, headers))
            }
        }
    }
}

impl<J, H, I> IntoResponse for UnionResponse<J, H, I>
where
    J: Serialize,
    H: IntoResponse,
    I: IntoIterator<Item = (HeaderName, HeaderValue)>,
{
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Json((code, json, headers)) => {
                let mut response = Json::from(json).into_response();
                *response.status_mut() = code;
                response.headers_mut().extend(headers);
                response
            }
            Self::Html((code, html, headers)) => {
                let mut response = Html::from(html).into_response();
                *response.status_mut() = code;
                response.headers_mut().extend(headers);
                response
            }
        }
    }
}

impl IntoUnionResponse<ErrorAsJson, Markup> for ApiError {
    fn into_json(self) -> (StatusCode, ErrorAsJson, Empty<(HeaderName, HeaderValue)>) {
        let (status, json) = self.into_json();
        (status, json.0, empty())
    }

    fn into_html(self) -> (StatusCode, Markup, Empty<(HeaderName, HeaderValue)>) {
        let (status, html) = self.into_html();
        (status, html.0, empty())
    }
}

