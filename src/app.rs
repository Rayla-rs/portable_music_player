use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::dma::ReadBuffer;
use log::info;
use pmp_config::Library;

use crate::{
    fs::{decode, FileSystem},
    input::Receiver,
    player::Player,
};

pub async fn run<'a, 'b, 'ch, TXBUF: ReadBuffer>(
    _spawner: Spawner,
    fs: &'a FileSystem<'a>,
    mut player: Player<'a, 'b, TXBUF>,
    // _input_receiver: Receiver<'ch>,
) -> ! {
    info!("Run App");
    let file = esp_println::dbg!(fs.open_file("library.post")).unwrap();
    loop {}
    let lib: Library = decode(fs.open_file("library.post").unwrap()).unwrap();

    // loop {}
    loop {
        // let a = spawner.spawn(test());
        let ply = lib.playlists[0].tracks.get(0).unwrap();
        let decoder = fs.open_track(&ply).unwrap();
        player.play(decoder);
        player.next().await.unwrap();

        Timer::after_nanos(100).await;
    }
}

#[embassy_executor::task]
async fn test() {}
