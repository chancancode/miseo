//! Parsed domain spec types used across CLI, tasks, and workspace code.

mod backend;
mod runtime;
mod tool;

pub use backend::Backend;
pub use runtime::{Runtime, RuntimePins, RuntimeSpec};
pub use tool::{ToolId, ToolLayout, ToolSpec, VariantLayout};
