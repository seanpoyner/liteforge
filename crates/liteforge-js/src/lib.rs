#![deny(clippy::all)]
#![allow(dead_code)] // FFI exports via napi macros aren't visible to Rust's dead code analysis

#[macro_use]
extern crate napi_derive;

mod agents;
mod automation;
mod chunking;
mod client;
mod conversation;
mod error;
mod evals;
mod events;
mod guardrails;
mod hitl;
mod hooks;
mod images;
mod knowledge;
mod mcp;
mod observability;
mod orchestration;
mod otel;
mod pipelines;
mod prompts;
mod rag;
mod retry;
mod scheduler;
mod skills;
mod streaming;
mod tools;
mod types;
