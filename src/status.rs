use crate::util::ATracked;
pub use macos_spotify::{Spotify as SpotifyClient, State as PlaybackStatus};
use std::fmt;
use std::sync::Arc;

pub struct Track {
    artist: ATracked<Option<String>>,
    id: ATracked<Option<String>>,
    name: ATracked<Option<String>>,
    album: ATracked<Option<String>>,
    album_artist: ATracked<Option<String>>,
    artwork_url: ATracked<Option<String>>,
    disk_number: ATracked<Option<i32>>,
    duration: ATracked<Option<i32>>,
    url: ATracked<Option<String>>,
}

impl Default for Track {
    fn default() -> Self {
        Track {
            artist: Default::default(),
            id: Default::default(),
            name: Default::default(),
            album: Default::default(),
            album_artist: Default::default(),
            artwork_url: Default::default(),
            disk_number: Default::default(),
            duration: Default::default(),
            url: Default::default(),
        }
    }
}

impl Track {
    pub fn artist(&self) -> Arc<Option<String>> {
        self.artist.get()
    }

    pub fn set_artist(&self, value: Option<String>) {
        self.artist.set(value)
    }

    pub fn id(&self) -> Arc<Option<String>> {
        self.id.get()
    }

    pub fn set_id(&self, value: Option<String>) {
        self.id.set(value)
    }

    pub fn name(&self) -> Arc<Option<String>> {
        self.name.get()
    }

    pub fn set_name(&self, value: Option<String>) {
        self.name.set(value)
    }

    pub fn album(&self) -> Arc<Option<String>> {
        self.album.get()
    }

    pub fn set_album(&self, value: Option<String>) {
        self.album.set(value)
    }

    pub fn album_artist(&self) -> Arc<Option<String>> {
        self.album_artist.get()
    }

    pub fn set_album_artist(&self, value: Option<String>) {
        self.album_artist.set(value)
    }

    pub fn artwork_url(&self) -> Arc<Option<String>> {
        self.artwork_url.get()
    }

    pub fn set_artwork_url(&self, value: Option<String>) {
        self.artwork_url.set(value)
    }

    pub fn disk_number(&self) -> Arc<Option<i32>> {
        self.disk_number.get()
    }

    pub fn set_disk_number(&self, value: Option<i32>) {
        self.disk_number.set(value)
    }

    pub fn duration(&self) -> Arc<Option<i32>> {
        self.duration.get()
    }

    pub fn set_duration(&self, value: Option<i32>) {
        self.duration.set(value)
    }

    pub fn url(&self) -> Arc<Option<String>> {
        self.url.get()
    }

    pub fn set_url(&self, value: Option<String>) {
        self.url.set(value)
    }

    pub fn has_changed(&self) -> bool {
        self.artist.has_changed()
            || self.id.has_changed()
            || self.name.has_changed()
            || self.album.has_changed()
            || self.album_artist.has_changed()
            || self.artwork_url.has_changed()
            || self.disk_number.has_changed()
            || self.duration.has_changed()
            || self.url.has_changed()
    }

    pub fn reset(&self) {
        self.artist.reset();
        self.id.reset();
        self.name.reset();
        self.album.reset();
        self.album_artist.reset();
        self.artwork_url.reset();
        self.disk_number.reset();
        self.duration.reset();
        self.url.reset();
    }
}

impl fmt::Debug for Track {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Track")
            .field("artist", self.artist().as_ref())
            .field("id", self.id().as_ref())
            .field("name", self.name().as_ref())
            .field("album", self.album().as_ref())
            .field("album_artist", self.album_artist().as_ref())
            .field("artwork_url", self.artwork_url().as_ref())
            .field("disk_number", self.disk_number().as_ref())
            .field("duration", self.duration().as_ref())
            .field("url", self.url().as_ref())
            .finish()
    }
}

unsafe impl Send for Track {}
unsafe impl Sync for Track {}

pub struct SpotifyStatus {
    track: Arc<Track>,
    playback_status: ATracked<PlaybackStatus>,
    shuffling: ATracked<Option<bool>>,
    repeating: ATracked<Option<bool>>,
    position: ATracked<Option<f64>>,
    volume: ATracked<Option<i32>>,
}

impl SpotifyStatus {
    pub fn playback_status(&self) -> PlaybackStatus {
        *self.playback_status.get()
    }

    pub fn set_playback_status(&self, value: PlaybackStatus) {
        self.playback_status.set(value);
    }

    pub fn is_shuffling(&self) -> Option<bool> {
        *self.shuffling.get()
    }

    pub fn set_shuffling(&self, value: Option<bool>) {
        self.shuffling.set(value);
    }

    pub fn is_repeating(&self) -> Option<bool> {
        *self.repeating.get()
    }

    pub fn set_repeating(&self, value: Option<bool>) {
        self.repeating.set(value);
    }

    pub fn position(&self) -> Option<f64> {
        *self.position.get()
    }

    pub fn set_position(&self, value: Option<f64>) {
        self.position.set(value);
    }

    pub fn volume(&self) -> Option<i32> {
        *self.volume.get()
    }

    pub fn set_volume(&self, value: Option<i32>) {
        self.volume.set(value);
    }

    pub fn track(&self) -> Arc<Track> {
        self.track.clone()
    }

    pub fn has_changed(&self) -> bool {
        self.track.has_changed()
            || self.playback_status.has_changed()
            || self.shuffling.has_changed()
            || self.repeating.has_changed()
            || self.position.has_changed()
            || self.volume.has_changed()
    }

    pub fn reset(&self) {
        self.track.reset();
        self.playback_status.reset();
        self.shuffling.reset();
        self.repeating.reset();
        self.position.reset();
        self.volume.reset();
    }

    pub fn update(&self, spotify: &SpotifyClient) -> std::io::Result<()> {
        match self._update(spotify) {
            Err(err) => {
                if let Some(code) = err.raw_os_error() {
                    if code == -600 || code == -609 {
                        self.set_playback_status(PlaybackStatus::STOPPED);
                        self.set_volume(None);
                        self.set_shuffling(None);
                        self.set_repeating(None);
                        self.set_position(None);

                        let track = self.track();

                        track.set_artist(None);
                        track.set_id(None);
                        track.set_name(None);
                        track.set_album(None);
                        track.set_album_artist(None);
                        track.set_artwork_url(None);
                        track.set_disk_number(None);
                        track.set_duration(None);
                        track.set_url(None);

                        Ok(())
                    } else {
                        Err(err)
                    }
                } else {
                    Err(err)
                }
            }
            Ok(()) => Ok(()),
        }
    }

    fn _update(&self, spotify: &SpotifyClient) -> std::io::Result<()> {
        let playback_status = spotify.state()?.unwrap_or(PlaybackStatus::STOPPED);
        let volume;
        let mut shuffling = None;
        let mut repeating = None;
        let mut position = None;

        let mut artist = None;
        let mut id = None;
        let mut name = None;
        let mut album = None;
        let mut album_artist = None;
        let mut artwork_url = None;
        let mut disk_number = None;
        let mut duration = None;
        let mut url = None;

        if playback_status != PlaybackStatus::STOPPED {
            shuffling = spotify.is_shuffling()?;
            repeating = spotify.is_repeating()?;
            position = spotify.pos()?;
            volume = spotify.volume()?;

            let track = spotify.track()?;

            if track.is_some() {
                let track = track.unwrap();
                artist = track.artist()?;
                id = track.id()?;
                name = track.name()?;
                album = track.album()?;
                album_artist = track.album_artist()?;
                artwork_url = track.artwork_url()?;
                disk_number = track.disk_number()?;
                duration = track.duration()?;
                url = track.url()?;
            }
        } else {
            volume = spotify.volume()?;
        }

        self.set_playback_status(playback_status);
        self.set_volume(volume);
        self.set_shuffling(shuffling);
        self.set_repeating(repeating);
        self.set_position(position);

        let track = self.track();

        track.set_artist(artist);
        track.set_id(id);
        track.set_name(name);
        track.set_album(album);
        track.set_album_artist(album_artist);
        track.set_artwork_url(artwork_url);
        track.set_disk_number(disk_number);
        track.set_duration(duration);
        track.set_url(url);

        Ok(())
    }
}

impl fmt::Debug for SpotifyStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SpotifyStatus")
            .field("track", self.track().as_ref())
            .field("playback_status", &self.playback_status())
            .field("shuffling", &self.is_shuffling().as_ref())
            .field("repeating", &self.is_repeating().as_ref())
            .field("position", &self.position().as_ref())
            .field("volume", &self.volume().as_ref())
            .finish()
    }
}

impl Default for SpotifyStatus {
    fn default() -> Self {
        SpotifyStatus {
            track: Arc::new(Default::default()),
            playback_status: ATracked::new(PlaybackStatus::STOPPED),
            shuffling: Default::default(),
            repeating: Default::default(),
            position: Default::default(),
            volume: Default::default(),
        }
    }
}

unsafe impl Send for SpotifyStatus {}
unsafe impl Sync for SpotifyStatus {}
