use core::{fmt::Debug, future::Future};
use embassy_executor::Spawner;
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex},
    channel::Sender,
};
use esp_hal::gpio::{Input, InputConfig, InputPin};
use heapless::Vec;

const INPUT_CHANNEL_CAPACITY: usize = 8;
pub type Channel =
    embassy_sync::channel::Channel<CriticalSectionRawMutex, InputEvent, INPUT_CHANNEL_CAPACITY>;
pub type Receiver<'ch> = embassy_sync::channel::Receiver<
    'ch,
    CriticalSectionRawMutex,
    InputEvent,
    INPUT_CHANNEL_CAPACITY,
>;

pub const fn create_input_channel() -> Channel {
    Channel::new()
}

/// Supported Input Events
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    Up,
    Down,
    Enter,
    Back,
    IncrementVolume,
    DecrementVolume,
}

/// Input peripheral wrapper that can be polled for a event
#[derive(Debug)]
pub struct Button<'a, Event: Copy + Clone + Debug> {
    input: Input<'a>,
    event: Event,
}

impl<'a, Event: Copy + Clone + Debug> Button<'a, Event> {
    fn new(pin: impl InputPin + 'a, event: Event) -> Self {
        Self {
            input: Input::new(pin, InputConfig::default()),
            event: event,
        }
    }

    async fn poll(&mut self) -> Event {
        self.input.wait_for_falling_edge().await;
        self.event
    }
}

/// Spawn an input task that will send messages through the channel when Button inputs are received
pub fn spawn_input_task(
    spawner: &Spawner,
    channel: &'static Channel,
    up: impl InputPin + 'static,
    down: impl InputPin + 'static,
    enter: impl InputPin + 'static,
    back: impl InputPin + 'static,
    increment_volume: impl InputPin + 'static,
    decrement_volume: impl InputPin + 'static,
) -> Receiver<'static> {
    spawner.must_spawn(input_task(
        channel.sender().clone(),
        [
            Button::new(up, InputEvent::Up),
            Button::new(down, InputEvent::Down),
            Button::new(enter, InputEvent::Enter),
            Button::new(back, InputEvent::Back),
            Button::new(increment_volume, InputEvent::IncrementVolume),
            Button::new(decrement_volume, InputEvent::DecrementVolume),
        ],
    ));
    channel.receiver()
}

#[embassy_executor::task(pool_size = 4)]
async fn input_task(
    sender: Sender<'static, CriticalSectionRawMutex, InputEvent, INPUT_CHANNEL_CAPACITY>,
    mut buttons: [Button<'static, InputEvent>; 6],
) -> ! {
    button_task(sender, &mut buttons).await
}

async fn button_task<'a, const N: usize, const COUNT: usize, Event: Copy + Clone + Debug>(
    sender: Sender<'a, impl RawMutex, Event, N>,
    buttons: &'a mut [Button<'a, Event>; COUNT],
) -> ! {
    loop {
        sender
            .send(
                embassy_futures::select::select_array(create_futures(buttons))
                    .await
                    .0,
            )
            .await;
    }
}

fn create_futures<'a: 'b, 'b, const COUNT: usize, Event: Copy + Clone + Debug>(
    buttons: &'b mut [Button<'a, Event>; COUNT],
) -> [impl Future<Output = Event> + use<'a, 'b, COUNT, Event>; COUNT] {
    unsafe {
        buttons
            .iter_mut()
            .map(|b| b.poll())
            .collect::<Vec<_, COUNT>>()
            .into_array()
            .unwrap_unchecked()
    }
}
