//! # intear-dex-aggregator-tool-box
//! Typed client + near-kit execution helpers for the intear dex-aggregator.
//!
//! Quote with [`lib::helper::dex_aggregator_client::DEX_AGGREGATOR_CLIENT`],
//! execute with [`fun::dex_aggregator::dex_aggregator_execute_route_fun::execute_route`].
//!
//! copyright 2026 by sleet.near
// =================================================
#![allow(non_camel_case_types)]
// =================================================
pub mod lib {
    /// Reusable dex-aggregator client configuration.
    pub mod helper;
    /// Shared response / data shapes.
    pub mod types;
}
// functions
pub mod fun;
// =================================================
// =================================================
// copyright 2026 by sleet.near
