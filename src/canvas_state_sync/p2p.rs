use futures::stream::StreamExt;
use libp2p::{gossipsub, mdns, noise, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::{io, select};
use bincode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::canvas_state_sync::sync_types::MessageType;

const MAX_DATA_TRANSFER_SIZE: usize = 1024*1024;

#[derive(NetworkBehaviour)]
pub struct TestBehavior {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub async fn p2p(
    mut gui_receiver: mpsc::Receiver<MessageType>,
    p2p_sender: mpsc::Sender<MessageType>,
    running: Arc<AtomicBool>
) {
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_behaviour(|key| {
            let message_id_fn = |_message: &gossipsub::Message| {
                // Note: Maybe the message should be used for the ID generation, but we'll see in the future.
                // This seems to work for now
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");

                // Use the number of seconds and nanoseconds as the unique identifier
                let timestamp = now.as_secs() as u64 * 1_000_000_000 + now.subsec_nanos() as u64;

                // Create a MessageId based on the timestamp
                gossipsub::MessageId::from(timestamp.to_string())
            };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .max_transmit_size(MAX_DATA_TRANSFER_SIZE)
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))
                .unwrap();

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )
            .unwrap();

            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
                    .unwrap();
            Ok(TestBehavior { gossipsub, mdns })
        })
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    let topic = gossipsub::IdentTopic::new("test-net");
    swarm.behaviour_mut().gossipsub.subscribe(&topic).unwrap();

    // let mut stdin = io::BufReader::new(io::stdin()).lines();
    swarm
        .listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap())
        .unwrap();
    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }
        
        select! {
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(TestBehaviorEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(TestBehaviorEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(TestBehaviorEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: _peer_id,
                    message_id: _id,
                    message,
                })) => {
                    let deserialized: Result<MessageType, _> = bincode::deserialize(&message.data);

                    if let Ok(msg) = deserialized {
                        // TODO: figure out how would error handling even work in this case
                        let _result = p2p_sender.send(msg).await;
                    }
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                }
                _ => {}
            },
            Some(message) = gui_receiver.recv() => {
                let serialized_message = bincode::serialize(&message).expect("failed to serialise");

                if let Err(e) = swarm
                    .behaviour_mut().gossipsub
                    .publish(topic.clone(), serialized_message) {
                }
            }
        }
    }
}
