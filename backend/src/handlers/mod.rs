//! Request handlers grouped by concern.
//!
//! Each sub-module owns the handlers for one logical area.
//! Routing wires them together in [`crate::routes`].

pub mod health;
