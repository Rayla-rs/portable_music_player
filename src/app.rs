use embassy_executor::SendSpawner;
use pmp_config::Library;

use crate::{
    fs::{decode, FileSystem},
    player::Sink,
};

/// Main task
#[embassy_executor::task]
pub async fn run(
    send_spawner: SendSpawner,
    fs: &'static FileSystem<'static>,
    player: Sink<'static>,
) -> ! {
    let lib: Library = decode(fs.open_file("config.toml").unwrap()).unwrap();
    loop {
        let a = send_spawner.spawn(test());

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

#[embassy_executor::task]
async fn test() {}
