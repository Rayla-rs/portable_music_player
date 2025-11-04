use embassy_executor::SendSpawner;
use pmp_config::Library;

use crate::{
    fs::{decode, FileSystem},
    Player,
};

/// Main task
#[embassy_executor::task]
async fn run(
    send_spawner: SendSpawner,
    fs: &'static FileSystem<'static>,
    player: Player<'static>,
) -> ! {
    let lib: Library = decode(fs.open_file("config.toml").unwrap()).unwrap();
    loop {
        fs.open_file(
            lib.playlists
                .first()
                .unwrap()
                .tracks
                .first()
                .unwrap()
                .path
                .as_str(),
        )
        .unwrap();
    }
}
