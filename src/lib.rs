#![deny(elided_lifetimes_in_paths, single_use_lifetimes)]

mod app;
mod storage_buffer;
mod texture;

pub use app::*;
pub use storage_buffer::*;
pub use texture::*;

pub const CHUNK_SIZE: u32 = 4;
