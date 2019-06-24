use std::rc::Rc;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex, RwLock,
};
use std::thread::{spawn, JoinHandle};

use std::collections::HashMap;

use crate::AppState;

use crate::status::PlaybackStatus;

use dbus::arg::{RefArg, Variant};
use dbus::stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;
use dbus::tree::{Access, Factory};
use dbus::{Path, SignalArgs};

#[derive(PartialEq, Eq)]
enum MprisCommand {
    Ok,
    Tick,
    Stop,
}

struct MprisInternal {
    app_state: Arc<AppState>,
    handle: JoinHandle<()>,
    tx: Sender<MprisCommand>,
    rx: Receiver<MprisCommand>,
}

impl MprisInternal {
    pub fn new(state: Arc<AppState>) -> MprisInternal {
        let (tx1, rx1) = channel::<MprisCommand>();
        let (tx2, rx2) = channel::<MprisCommand>();

        let moving_state = state.clone();

        let handle = spawn(move || {
            run_server(moving_state, tx2, rx1);
        });

        MprisInternal {
            app_state: state,
            handle,
            tx: tx1,
            rx: rx2,
        }
    }

    pub fn update(&self) {
        self.tx.send(MprisCommand::Tick).unwrap();
        loop {
            if let Ok(msg) = self.rx.recv() {
                if msg == MprisCommand::Ok {
                    {
                        let mut x = self.app_state.mpris().locker.lock().unwrap();
                        *x = ();
                    }
                    self.app_state.reset();
                    break;
                }
            }
        }
    }
}

pub struct Mpris {
    inner: RwLock<Option<MprisInternal>>,
    locker: Mutex<()>,
}

impl Mpris {
    pub fn new() -> Mpris {
        Mpris {
            inner: RwLock::new(None),
            locker: Mutex::new(()),
        }
    }

    pub fn run(&self, state: Arc<AppState>) {
        if !self.is_running() {
            let inner = MprisInternal::new(state);

            {
                let mut v = self.inner.write().unwrap();
                *v = Some(inner);
            }
        }
    }

    pub fn is_running(&self) -> bool {
        (*self.inner.read().unwrap()).is_some()
    }

    pub fn update(&self) {
        if self.is_running() {
            self.inner.read().unwrap().as_ref().unwrap().update();
        }
    }
}

unsafe impl Send for Mpris {}
unsafe impl Sync for Mpris {}

fn get_metadata(state: Arc<AppState>) -> HashMap<String, Variant<Box<RefArg>>> {
    let mut hm: HashMap<String, Variant<Box<RefArg>>> = HashMap::new();

    let track = state.spotify_status().track();

    hm.insert(
        "mpris:trackid".to_string(),
        Variant(Box::new(
            track
                .id()
                .as_ref()
                .as_ref()
                .map(|v| v.clone())
                .unwrap_or_default(),
        )),
    );

    hm.insert(
        "mpris:length".to_string(),
        Variant(Box::new(
            track
                .duration()
                .as_ref()
                .as_ref()
                .map(|v| v * 1_000)
                .unwrap_or_default(),
        )),
    );

    hm.insert(
        "mpris:artUrl".to_string(),
        Variant(Box::new(
            track
                .artwork_url()
                .as_ref()
                .as_ref()
                .map(|v| v.clone())
                .unwrap_or_default(),
        )),
    );

    hm.insert(
        "xesam:album".to_string(),
        Variant(Box::new(
            track
                .album()
                .as_ref()
                .as_ref()
                .map(|v| v.clone())
                .unwrap_or_default(),
        )),
    );

    hm.insert(
        "xesam:albumArtist".to_string(),
        Variant(Box::new(
            track
                .album_artist()
                .as_ref()
                .as_ref()
                .map(|v| v.clone())
                .unwrap_or_default(),
        )),
    );

    hm.insert(
        "xesam:artist".to_string(),
        Variant(Box::new(
            track
                .artist()
                .as_ref()
                .as_ref()
                .map(|v| v.clone())
                .unwrap_or_default(),
        )),
    );

    hm.insert(
        "xesam:discNumber".to_string(),
        Variant(Box::new(
            track
                .disk_number()
                .as_ref()
                .as_ref()
                .map(|v| *v)
                .unwrap_or_default(),
        )),
    );

    hm.insert(
        "xesam:title".to_string(),
        Variant(Box::new(
            track
                .name()
                .as_ref()
                .as_ref()
                .map(|v| v.clone())
                .unwrap_or_default(),
        )),
    );

    hm.insert("xesam:trackNumber".to_string(), Variant(Box::new(0)));

    hm.insert(
        "xesam:url".to_string(),
        Variant(Box::new(
            track
                .url()
                .as_ref()
                .as_ref()
                .map(|v| v.clone())
                .unwrap_or_default(),
        )),
    );

    hm
}

fn get_playbackstatus(state: Arc<AppState>) -> String {
    match state.spotify_status().playback_status() {
        PlaybackStatus::STOPPED => "Stopped",
        PlaybackStatus::PLAYING => "Playing",
        PlaybackStatus::PAUSED => "Paused",
    }
    .to_string()
}

fn get_loopstatus(state: Arc<AppState>) -> String {
    match state.spotify_status().is_repeating() {
        None | Some(false) => "None",
        Some(true) => "Playlist",
    }
    .to_string()
}

fn get_shuffle(state: Arc<AppState>) -> bool {
    match state.spotify_status().is_shuffling() {
        None | Some(false) => false,
        Some(true) => true,
    }
}

fn run_server(state: Arc<AppState>, tx: Sender<MprisCommand>, rx: Receiver<MprisCommand>) {
    let conn = Rc::new(
        dbus::Connection::get_private(dbus::BusType::Session).expect("Failed to connect to dbus"),
    );
    conn.register_name(
        "org.mpris.MediaPlayer2.spotify",
        dbus::NameFlag::ReplaceExisting as u32,
    )
    .expect("Failed to register dbus player name");

    let f = Factory::new_fn::<()>();

    let property_canquit = f
        .property::<bool, _>("CanQuit", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(false); // TODO
            Ok(())
        });

    let property_canraise = f
        .property::<bool, _>("CanRaise", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(false); // TODO
            Ok(())
        });

    let property_cansetfullscreen = f
        .property::<bool, _>("CanSetFullscreen", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(false); // TODO
            Ok(())
        });

    let property_hastracklist = f
        .property::<bool, _>("HasTrackList", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(false);
            Ok(())
        });

    let property_identity = f
        .property::<String, _>("Identity", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append("spotify".to_string());
            Ok(())
        });

    let property_urischemes = f
        .property::<Vec<String>, _>("SupportedUriSchemes", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(vec!["spotify".to_string()]);
            Ok(())
        });

    let property_mimetypes = f
        .property::<Vec<String>, _>("SupportedMimeTypes", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(Vec::new() as Vec<String>);
            Ok(())
        });

    let interface = f
        .interface("org.mpris.MediaPlayer2", ())
        .add_p(property_canquit)
        .add_p(property_canraise)
        .add_p(property_cansetfullscreen)
        .add_p(property_hastracklist)
        .add_p(property_identity)
        .add_p(property_urischemes)
        .add_p(property_mimetypes);

    let property_playbackstatus = {
        let state = state.clone();
        f.property::<String, _>("PlaybackStatus", ())
            .access(Access::Read)
            .on_get(move |iter, _| {
                iter.append(get_playbackstatus(state.clone()));
                Ok(())
            })
    };

    let property_loopstatus = {
        let state = state.clone();
        let state2 = state.clone();
        f.property::<String, _>("LoopStatus", ())
            .access(Access::ReadWrite)
            .on_get(move |iter, _| {
                iter.append(get_loopstatus(state.clone()));
                Ok(())
            })
            .on_set(move |iter, _| {
                match iter.get() {
                    Some("None") => {
                        state2.client().set_repeating(false);
                    }
                    Some("Playlist") => {
                        state2.client().set_repeating(true);
                    }
                    _ => {}
                };
                Ok(())
            })
    };

    let property_metadata = {
        let state = state.clone();
        f.property::<HashMap<String, Variant<Box<RefArg>>>, _>("Metadata", ())
            .access(Access::Read)
            .on_get(move |iter, _| {
                iter.append(get_metadata(state.clone()));
                Ok(())
            })
    };

    let property_position = {
        let state = state.clone();
        f.property::<i64, _>("Position", ())
            .access(Access::Read)
            .on_get(move |iter, _| {
                iter.append(
                    state
                        .spotify_status()
                        .position()
                        .map(|v| (v * 1_000_000.0).round() as i64)
                        .unwrap_or_default(),
                );
                Ok(())
            })
    };

    let property_volume = {
        let state = state.clone();
        let state2 = state.clone();
        f.property::<f64, _>("Volume", ())
            .access(Access::ReadWrite)
            .on_get(move |iter, _| {
                iter.append(
                    state
                        .spotify_status()
                        .volume()
                        .map(|v| (v as f64) / 100.0)
                        .unwrap_or_default(),
                );
                Ok(())
            })
            .on_set(move |iter, _| {
                if let Some(vol) = iter.get::<f64>() {
                    state2.client().set_volume((vol * 100.0).round() as i32);
                }
                Ok(())
            })
    };

    let property_rate = f
        .property::<f64, _>("Rate", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(1.0);
            Ok(())
        });

    let property_minrate = f
        .property::<f64, _>("MinimumRate", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(1.0);
            Ok(())
        });

    let property_maxrate = f
        .property::<f64, _>("MaximumRate", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(1.0);
            Ok(())
        });

    let property_canplay = f
        .property::<bool, _>("CanPlay", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(true);
            Ok(())
        });

    let property_canpause = f
        .property::<bool, _>("CanPause", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(true);
            Ok(())
        });

    let property_canseek = f
        .property::<bool, _>("CanSeek", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(false); // TODO
            Ok(())
        });

    let property_cancontrol = f
        .property::<bool, _>("CanControl", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(true);
            Ok(())
        });

    let property_cangonext = f
        .property::<bool, _>("CanGoNext", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(false);
            Ok(())
        });

    let property_cangoprevious = f
        .property::<bool, _>("CanGoPrevious", ())
        .access(Access::Read)
        .on_get(|iter, _| {
            iter.append(false);
            Ok(())
        });

    let property_shuffle = {
        let state = state.clone();
        let state2 = state.clone();
        f.property::<bool, _>("Shuffle", ())
            .access(Access::ReadWrite)
            .on_get(move |iter, _| {
                iter.append(get_shuffle(state.clone()));
                Ok(())
            })
            .on_set(move |iter, _| {
                if let Some(value) = iter.get() {
                    state2.client().set_repeating(value);
                }
                Ok(())
            })
    };

    let method_playpause = {
        let state = state.clone();
        f.method("PlayPause", (), move |m| {
            state.client().play_pause();
            Ok(vec![m.msg.method_return()])
        })
    };

    let method_play = {
        let state = state.clone();
        f.method("Play", (), move |m| {
            state.client().play();
            Ok(vec![m.msg.method_return()])
        })
    };

    let method_pause = {
        let state = state.clone();
        f.method("Pause", (), move |m| {
            state.client().pause();
            Ok(vec![m.msg.method_return()])
        })
    };

    let method_stop = {
        let state = state.clone();
        f.method("Stop", (), move |m| {
            state.client().pause();
            Ok(vec![m.msg.method_return()])
        })
    };

    let method_next = {
        let state = state.clone();
        f.method("Next", (), move |m| {
            state.client().next();
            Ok(vec![m.msg.method_return()])
        })
    };

    let method_previous = {
        let state = state.clone();
        f.method("Previous", (), move |m| {
            state.client().prev();
            Ok(vec![m.msg.method_return()])
        })
    };

    let interface_player = f
        .interface("org.mpris.MediaPlayer2.Player", ())
        .add_p(property_playbackstatus)
        .add_p(property_loopstatus)
        .add_p(property_metadata)
        .add_p(property_position)
        .add_p(property_volume)
        .add_p(property_rate)
        .add_p(property_minrate)
        .add_p(property_maxrate)
        .add_p(property_canplay)
        .add_p(property_canpause)
        .add_p(property_canseek)
        .add_p(property_cancontrol)
        .add_p(property_cangonext)
        .add_p(property_cangoprevious)
        .add_p(property_shuffle)
        .add_m(method_playpause)
        .add_m(method_play)
        .add_m(method_pause)
        .add_m(method_stop)
        .add_m(method_next)
        .add_m(method_previous);

    let tree = f.tree(()).add(
        f.object_path("/org/mpris/MediaPlayer2", ())
            .introspectable()
            .add(interface)
            .add(interface_player),
    );

    tree.set_registered(&conn, true)
        .expect("failed to register tree");

    conn.add_handler(tree);

    loop {
        if let Some(m) = conn.incoming(200).next() {
            println!("Unhandled dbus message: {:?}", m);
        }

        let mut update = false;

        match rx.try_recv() {
            Ok(cmd) => {
                match cmd {
                    MprisCommand::Ok => {}
                    MprisCommand::Tick => match state.mpris().locker.lock() {
                        Ok(mut guard) => {
                            match tx.send(MprisCommand::Ok) {
                                Ok(_) => {
                                    update = state.spotify_status().has_changed();
                                }
                                Err(_) => {}
                            };
                            *guard = ();
                        }
                        Err(_) => {}
                    },
                    MprisCommand::Stop => break,
                };
            }
            _ => {}
        }

        if update {
            let mut changed: PropertiesPropertiesChanged = Default::default();
            changed.interface_name = "org.mpris.MediaPlayer2.Player".to_string();

            let status = state.spotify_status();

            if status.track().has_changed() {
                changed.changed_properties.insert(
                    "Metadata".to_string(),
                    Variant(Box::new(get_metadata(state.clone()))),
                );
            }

            changed.changed_properties.insert(
                "PlaybackStatus".to_string(),
                Variant(Box::new(get_playbackstatus(state.clone()))),
            );

            changed.changed_properties.insert(
                "LoopStatus".to_string(),
                Variant(Box::new(get_playbackstatus(state.clone()))),
            );

            changed.changed_properties.insert(
                "Shuffle".to_string(),
                Variant(Box::new(get_shuffle(state.clone()))),
            );

            changed.changed_properties.insert(
                "Volume".to_string(),
                Variant(Box::new(
                    state
                        .spotify_status()
                        .volume()
                        .map(|v| (v as f64) / 100.0)
                        .unwrap_or_default(),
                )),
            );

            changed.changed_properties.insert(
                "Position".to_string(),
                Variant(Box::new(
                    state
                        .spotify_status()
                        .position()
                        .map(|v| (v * 1_000_000.0).round() as i64)
                        .unwrap_or_default(),
                )),
            );

            conn.send(
                changed.to_emit_message(&Path::new("/org/mpris/MediaPlayer2".to_string()).unwrap()),
            )
            .unwrap();
        }
    }
}
