use embassy_executor::Spawner;
use esp_hal::dma::ReadBuffer;
use pmp_config::Library;

use crate::{
    fs::{decode, FileSystem},
    input::Receiver,
    player::Player,
};

pub async fn run<'a, 'b, 'ch, TXBUF: ReadBuffer>(
    _spawner: Spawner,
    fs: &'a FileSystem<'a>,
    mut player: Player<'a, TXBUF>,
    _input_receiver: Receiver<'ch>,
) -> ! {
    let lib: Library = decode(fs.open_file("config.toml").unwrap()).unwrap();
    loop {
        // let a = spawner.spawn(test());
        let ply = lib.playlists[0].tracks.get(0).unwrap();
        let decoder = fs.open_track(ply.clone()).unwrap();
        player.play(decoder);
        player.next().await.unwrap();
    }
}

#[embassy_executor::task]
async fn test() {}
