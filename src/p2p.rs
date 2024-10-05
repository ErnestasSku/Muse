use futures::stream::StreamExt;
use libp2p::{gossipsub, mdns, noise, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::{io, io::AsyncBufReadExt, select};
// use std::sync::mpsc;
use bincode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::communication::MessageType;

#[derive(NetworkBehaviour)]
pub struct TestBehavior {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub async fn p2p(
    mut gui_receiver: mpsc::Receiver<MessageType>,
    p2p_sender: mpsc::Sender<MessageType>,
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
            let message_id_fn = |message: &gossipsub::Message| {
                // let mut s = DefaultHasher::new();
                // message.data.hash(&mut s);
                // gossipsub::MessageId::from(s.finish().to_string())
                // Get the current time as a duration since the UNIX epoch
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
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(240)))
        .build();

    let topic = gossipsub::IdentTopic::new("test-net");
    swarm.behaviour_mut().gossipsub.subscribe(&topic).unwrap();

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    swarm
        .listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap())
        .unwrap();
    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();

    // let a = gui_receiver.recv();

    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                if let Err(e) = swarm
                    .behaviour_mut().gossipsub
                    .publish(topic.clone(), line.as_bytes()) {
                    println!("Publish error: {e:?}");
                }
            },
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
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => {
                    let deserialized: Result<MessageType, _> = bincode::deserialize(&message.data);

                    println!("Got message and deserialised");
                    if let Ok(msg) = deserialized {
                        let result = p2p_sender.send(msg).await;
                        println!("Message is send back to gui");
                    }

                    // match deserialized {
                    //     Ok(message) => {
                    //         match message {
                    //             MessageType::NewImage { bytes } => {
                    //                 println!("Received an image with {} bytes", bytes.len());
                    //             }
                    //             MessageType::CanvasState { state } => {
                    //                 // println!("Received a canvas state: {:?}", state);
                    //                 p2p_sender.send(value)
                    //             }
                    //             // MessageType::ConnectionRequest { msg } => {
                    //             //     println!("Received a connection request: {}", msg);
                    //             // }
                    //             // MessageType::ConnectionResponse { msg, connection_accepted } => {
                    //             //     println!(
                    //             //         "Received a connection response: {}, accepted: {}",
                    //             //         msg, connection_accepted
                    //             //     );
                    //             // }
                    //         }
                    //     }
                    //     Err(e) => {
                    //         println!("Failed to deserialize message: {:?}", e);
                    //     }
                    // }
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                }
                _ => {}
            },
            // Forgot we need to send the message to p2p, and receive it from it
            Some(message) = gui_receiver.recv() => {
                // match message {
                //     MessageType::NewImage { bytes } => todo!(),
                //     MessageType::CanvasState { state } => {
                //         if let Err(e) = swarm
                //             .behaviour_mut().gossipsub
                //             .publish
                //     },
                let serialized_message = bincode::serialize(&message).expect("failed to serialise");

                println!("SENDING FROM P2P");
                if let Err(e) = swarm
                    .behaviour_mut().gossipsub
                    .publish(topic.clone(), serialized_message) {
                    println!("Publish error: {e:?}");
                }
            }

            // Ok(message) = gui_receiver => {
                //
            // }
        }
    }
}
