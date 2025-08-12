// Enhanced window manager with drag and resize capabilities
pub mod basic_ops;
pub mod drag_resize;

// Re-export all functions from both modules
pub use basic_ops::*;
pub use drag_resize::*;