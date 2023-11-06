use std::collections::HashMap;
use std::hash::Hash;

use crate::inputs::Input;

/// A trait alias for what your [`InputBind`] must implement.
///
/// You don't need to implement [`InputBind`] you just need to implement [`PartialEq`], [`Eq`] and [`Hash`]
///
/// # Example
/// ```
/// #[derive(PartialEq, Eq, Hash)]
/// enum Binds {
///     Up,
///     Down,
///     Left,
///     Right,
/// }
/// ```
pub trait InputBind: PartialEq + Eq + Hash {}
impl<B> InputBind for B where B: PartialEq + Eq + Hash {}

/// A map of keys to their bindings.
#[derive(Debug)]
pub struct Bindings<B: InputBind> {
    key_map: HashMap<B, Vec<Input>>,
}

impl<B: InputBind> Default for Bindings<B> {
    fn default() -> Self {
        Self {
            key_map: HashMap::default(),
        }
    }
}

impl<B: InputBind> Bindings<B> {
    /// Bind a key to a binding
    pub fn bind(&mut self, key: Input, input: B) {
        let key_list = self.key_map.entry(input).or_default();
        if key_list.contains(&key) {
            return;
        }
        key_list.push(key);
    }

    /// Unbind a key and binding pair
    pub fn unbind(&mut self, key: &Input, input: B) {
        self.key_map.entry(input).or_default().retain(|k| k != key)
    }

    /// Transform an input into a list of its bound keys
    pub fn transform(&self, input: &B) -> &[Input] {
        self.key_map.get(input).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// An axis binding that combines two [`Bindings`] two form a 1 dimensional axis
///
/// Use [`crate::InputManager::axis`] to get a value from your axis bind or one of the multi dimension methods:
/// [`crate::InputManager::axis_n`] or [`crate::InputManager::axis_n_norm`]
#[derive(Debug)]
pub struct AxisBind<'a, B: InputBind> {
    /// The binding for the positive direction
    pub pos: &'a B,
    /// The binding for the negative direction
    pub neg: &'a B,
}

/// Convenience macro for declaring bindings quickly.
///
/// # Example
/// ```
/// use ru_input_helper::{Bindings, dry_binds};
///
/// #[derive(PartialEq, Eq, Hash)]
/// enum Binds {
///     Up,
///     Left,
///     Down,
///     Right,
/// }
///
/// use winit::event::VirtualKeyCode as VK;
/// use Binds::*;
/// let bindings: Bindings<Binds> = dry_binds!{
///     VK::W => Up,
///     VK::A => Left,
///     VK::S => Down,
///     VK::D => Right,
/// };
/// ```
#[macro_export]
macro_rules! dry_binds {
    ($($key:expr => $bind:expr),* $(,)?) => {{
        let mut binds = Bindings::default();
        $(binds.bind($key.into(), $bind));*;
        binds
    }}
}
