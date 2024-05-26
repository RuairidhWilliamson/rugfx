use winit::{
    event::MouseButton,
    keyboard::{KeyCode, PhysicalKey},
};

pub mod bindings;
pub mod input_manager;
pub mod inputs;
pub mod raw;
pub mod ticker;

/// Input represents any kind of user input
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Input {
    /// Keyboard button
    Key(PhysicalKey),
    /// Mouse button
    Mouse(winit::event::MouseButton),
}

impl From<PhysicalKey> for Input {
    fn from(value: PhysicalKey) -> Self {
        Self::Key(value)
    }
}

impl From<KeyCode> for Input {
    fn from(value: KeyCode) -> Self {
        Self::Key(PhysicalKey::Code(value))
    }
}

impl From<MouseButton> for Input {
    fn from(value: MouseButton) -> Self {
        Self::Mouse(value)
    }
}
