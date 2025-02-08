#![warn(clippy::pedantic)]
#![allow(
    clippy::must_use_candidate,
    clippy::uninlined_format_args,
    clippy::missing_errors_doc
)]

pub mod cli;
pub mod config;
pub mod driver;
pub mod error;
pub mod geo;
pub mod protocol;
pub mod units;
