use winit::event::VirtualKeyCode;

/// Key represents the different ways of referencing a key on a keyboard either a virtual key code or a scan code.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Key {
    /// Virtual key code
    Vk(VirtualKeyCode),
    /// Scan code
    Scan(u32),
}

impl From<VirtualKeyCode> for Key {
    fn from(value: VirtualKeyCode) -> Self {
        Key::Vk(value)
    }
}

impl From<u32> for Key {
    fn from(value: u32) -> Self {
        Key::Scan(value)
    }
}
