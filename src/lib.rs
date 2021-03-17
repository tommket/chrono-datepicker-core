#![forbid(unsafe_code)]

pub mod config;
pub mod dialog_view_type;
pub mod style_names;
pub mod utils;
pub mod viewed_date;

#[cfg(test)]
pub mod rstest_utils;

#[macro_use]
extern crate derive_getters;

#[macro_use]
extern crate derive_builder;
