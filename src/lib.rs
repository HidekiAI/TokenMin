#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod db;

pub mod engine;
pub mod models;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
