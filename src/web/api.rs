use std::{collections::HashMap, error::Error, sync::Arc};

use axum::{
    Json, Router, debug_handler,
    extract::{Path, Query, Request, State},
    http::{HeaderName, HeaderValue, StatusCode, header::LOCATION},
    response::{Html, IntoResponse},
    routing::get,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::Days;
use jsonwebtoken::{EncodingKey, Header, encode};
use maud::{Markup, Render, html};
use reqwest::header::{ACCESS_CONTROL_ALLOW_ORIGIN, SET_COOKIE};
use serde::{Deserialize, Serialize};
use serenity::all::User;
use sqlx::SqlitePool;
use strum::EnumString;

mod html;
mod json;
mod union;
