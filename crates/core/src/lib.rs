//! The review projects of papers-review, with no user interface: a folder,
//! what `review.toml` adds to it, and its files.

pub mod colour;
pub mod files;
pub mod project;

pub use colour::Colour;
pub use files::{Kind, Node};
pub use project::{Error, MANIFEST, Manifest, Project, REVIEW, ReviewFile};
pub use toml_edit::Datetime;
