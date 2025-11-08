use esp_hal::gpio::{interconnect::PeripheralInput, AnyPin, Input};

#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    Up,
    Down,
    Left,
    Right,
    Center,
}

pub struct AllInput<'a> {
    up: Input<'a>,
}

impl<'a> AllInput<'a> {
    pub fn new(
        up: impl PeripheralInput<'a>,
        down: impl PeripheralInput<'a>,
        left: impl PeripheralInput<'a>,
        right: impl PeripheralInput<'a>,
        center: impl PeripheralInput<'a>,
    ) -> Self {
        todo!()
        // Self {}
    }
}
