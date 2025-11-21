use embedded_menu::{
    items::{menu_item::SelectValue, MenuItem},
    MenuStyle, SelectValue,
};
use heapless::{Vec, VecView};
use pmp_config::{Playlist, Track};

use crate::input::Receiver;

struct ListState {
    index: usize,
}

impl ListState {
    // update
}

enum Item {
    Playlist(Playlist),
    Track(Track),
}

// item view

struct Menu<const N: usize> {
    elements: Vec<Item, N>,
    ptr: usize,
}

impl<const N: usize> Menu<N> {
    fn up(&mut self) {
        self.ptr = (self.ptr + 1)
            .checked_rem(self.elements.len())
            .unwrap_or_default()
    }
    fn down(&mut self) {
        self.ptr = match self.ptr {
            0 => self.elements.len().saturating_sub(1),
            _ => (self.ptr - 1)
                .checked_rem(self.elements.len())
                .unwrap_or_default(),
        }
    }
}

// struct ListItem {
//     timestamp: u32,
//     string: &'static str, // add scroll based on time since birth
// }

pub struct UI<'ch> {
    button_receiver: Receiver<'ch>,
    menus: heapless::Deque<Menu<1>, 3>,
}

impl<'ch> UI<'ch> {
    fn do_stuff(&mut self) {

        // self.menus.front()
    }
}

fn testing() {
    let mut a = embedded_menu::Menu::build("menu")
        .add_item("a", ">", |_| 1)
        .build();
    // a.
    // let mut b = embedded_menu::Menu::build("a")
}

#[derive(Clone, Copy, PartialEq)]
enum Command<'a> {
    Play,
    PlayTrack(&'a Track),
}

impl<'a> SelectValue for Command<'a> {
    // fn next(&mut self) {}
    fn marker(&self) -> &str {
        ""
    }
}

fn ply_menu(ply: Playlist) -> Playlist {
    let mut ls = [MenuItem::new("", ()).with_value_converter(|_| Command::Play)];
    let mut a = embedded_menu::Menu::build(ply.title.as_str())
        .add_item("Play", ">", |_| Command::Play)
        .add_menu_items(&mut ls)
        // .add_item("Songs", ply, |_| {})
        .build();
    match a.selected_value() {
        _ => ply,
    }
}

// struct WindowMenu<'a, Item, const SIZE: usize> {
// items: &'a VecView<&'a [Item]>,
// window: usize,
// offset: usize,
// }

fn play_track_item(track: &Track) -> MenuItem<&str, (), Command, true> {
    MenuItem::new(track.title.as_ref(), Command::PlayTrack(track))
}

fn track_menu(playlist: Playlist) {
    let b: Vec<&[Track], 20> = playlist.tracks.windows(2).collect();
    // b.as_view().;
    // let mut menu = embedded_menu::Menu::build("Tracks").add_menu_items(playlist.tracks);
}

// make methods to build each menu
// make seperate thing for displaying audio progress
// make seperate thing for fft

// what if i made a scrolling view over a contiguous array of elements
// or some sort of smart iterator that does not consume elements

// menu items -> ordered, has value, has converter, has display, has marker
