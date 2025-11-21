#![no_std]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![feature(slice_as_array)]

pub mod app;
pub mod fs;
pub mod input;
pub mod player;
mod ui;
mod visualizer;
