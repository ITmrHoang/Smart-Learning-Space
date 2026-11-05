//! `sls-api` — HTTP API (Axum). Tách `lib.rs` để integration test dựng router mà không chạy server.

pub mod app;
pub mod controllers;
pub mod extract;
pub mod middleware;
