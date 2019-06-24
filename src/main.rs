extern crate macos_spotify;

mod mpris;
mod status;
mod util;

use macos_spotify::Spotify as SpotifyClient;
use mpris::Mpris;
use status::SpotifyStatus;
use std::sync::Arc;

pub struct AppState {
    client: SpotifyClient,
    spotify_status: SpotifyStatus,
    mpris: Mpris,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            client: SpotifyClient::new(),
            spotify_status: Default::default(),
            mpris: Mpris::new(),
        }
    }

    pub fn client(&self) -> &SpotifyClient {
        &self.client
    }

    pub fn spotify_status(&self) -> &SpotifyStatus {
        &self.spotify_status
    }

    pub fn mpris(&self) -> &Mpris {
        &self.mpris
    }

    pub fn update(&self) -> std::io::Result<()> {
        self.spotify_status.update(&self.client)?;
        if self.has_changed() {
            self.mpris.update();
        }

        Ok(())
    }

    pub fn reset(&self) {
        self.spotify_status.reset();
    }

    pub fn has_changed(&self) -> bool {
        self.spotify_status.has_changed()
    }
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

fn readln() -> String {
    use std::io::{self, BufRead};

    let mut line = String::new();
    let stdin = io::stdin();
    stdin.lock().read_line(&mut line).unwrap();
    line
}

fn main() {
    let state = Arc::new(AppState::new());

    state.mpris().run(state.clone());

    loop {
        if let Err(err) = state.update() {
            println!("{}", err);
        }

        // readln();
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
}
