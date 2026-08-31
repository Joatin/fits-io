//! Struct for working with fits images

/// Reading and writing images stored tile-compressed inside a table.
pub mod compression;
mod group;
mod image;
mod image_data;
mod normalizer;

pub use self::group::Group;
pub(crate) use self::group::decode_group;
pub use self::image::Image;
pub use self::image_data::ImageData;
pub use self::normalizer::Normalizer;
