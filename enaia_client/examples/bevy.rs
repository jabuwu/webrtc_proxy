use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use enaia_client::EnaiaClient;
use rusty_enet::{Event, Host, HostSettings, Packet, PeerID};
use web_time::Instant;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Default)]
pub struct Update {
    host: Option<Arc<RwLock<Host<EnaiaClient>>>>,
    sent_time: Option<Instant>,
}

fn update(mut local: Local<Update>, keys: Res<Input<KeyCode>>) {
    let Update { host, sent_time } = &mut *local;
    let host = host.get_or_insert_with(|| {
        let mut host = Host::create(
            EnaiaClient::new(),
            HostSettings {
                peer_limit: 1,
                channel_limit: 2,
                ..Default::default()
            },
        )
        .unwrap();
        host.connect("http://127.0.0.1:14191".into(), 2, 0).unwrap();
        Arc::new(RwLock::new(host))
    });
    {
        let mut host = host.write().unwrap();
        while let Some(event) = host.service().unwrap() {
            match event {
                Event::Connect { peer, .. } => {
                    println!("Connection succeeded.");
                    let packet = Packet::reliable("hello world".as_bytes());
                    _ = peer.send(0, packet);
                }
                rusty_enet::Event::Receive { .. } => {
                    if let Some(sent_time) = &sent_time {
                        dbg!(sent_time.elapsed());
                    }
                }
                _ => {}
            }
        }
        if keys.just_pressed(KeyCode::Space) {
            let packet = Packet::reliable("hello world!!".as_bytes());
            _ = host
                .peer_mut(PeerID(0))
                .and_then(|peer| peer.send(0, packet));
            *sent_time = Some(Instant::now());
        }
    }
}
