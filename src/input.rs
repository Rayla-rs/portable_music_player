use core::future::Future;

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

#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    Up,
    Down,
    Left,
    Right,
    Center,
}

pub struct Button<'a> {
    input: Input<'a>,
    event: InputEvent,
}

impl<'a> Button<'a> {
    async fn process(&mut self) -> InputEvent {
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
) -> Receiver<'static> {
    spawner.must_spawn(input_task(
        channel.sender().clone(),
        [
            Button {
                input: Input::new(up, InputConfig::default()),
                event: InputEvent::Up,
            },
            Button {
                input: Input::new(down, InputConfig::default()),
                event: InputEvent::Down,
            },
        ],
    ));
    channel.receiver()
}

#[embassy_executor::task(pool_size = 4)]
async fn input_task(
    sender: Sender<'static, CriticalSectionRawMutex, InputEvent, INPUT_CHANNEL_CAPACITY>,
    mut buttons: [Button<'static>; 2],
) -> ! {
    input_task_inner(sender, &mut buttons).await
}

async fn input_task_inner<'a, const N: usize, const COUNT: usize>(
    sender: Sender<'a, impl RawMutex, InputEvent, N>,
    buttons: &'a mut [Button<'a>; COUNT],
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

fn create_futures<'a: 'b, 'b, const COUNT: usize>(
    buttons: &'b mut [Button<'a>; COUNT],
) -> [impl Future<Output = InputEvent> + use<'a, 'b, COUNT>; COUNT] {
    match buttons
        .iter_mut()
        .map(|b| b.process())
        .collect::<Vec<_, COUNT>>()
        .into_array()
    {
        Ok(value) => value,
        Err(_) => {
            unreachable!()
        }
    }
}
