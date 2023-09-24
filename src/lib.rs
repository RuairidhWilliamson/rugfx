#![warn(missing_docs, clippy::unwrap_used)]
//! Provides useful input manager for working with winit event loops

mod bindings;
mod input_manager;
mod key;
mod raw;
mod ticker;

pub use bindings::{AxisBind, Bindings, InputBind};
pub use input_manager::InputManager;
pub use key::Key;
pub use raw::RawInputManager;
pub use ticker::Ticker;
