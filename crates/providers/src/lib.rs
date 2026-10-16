//! Replaceable language-model providers and the local tool execution loop.
mod agent;
mod demo;
mod http;
pub use agent::*;
pub use demo::DemoProvider;
pub use http::HttpProvider;
