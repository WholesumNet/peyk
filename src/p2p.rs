use std::{
    time::Duration,
    hash::{
        Hash,
        Hasher
    },
    collections::hash_map::DefaultHasher
};
use libp2p::{
    core::{        
        muxing::StreamMuxerBox,
        transport::Boxed
    },
    tcp,
    yamux,
    noise,
    Transport,    
    gossipsub,
    identity,
    identify,
    request_response,
    kad, kad::store::MemoryStore,
    swarm::{
        Swarm,
        NetworkBehaviour,
        StreamProtocol,
    },
    SwarmBuilder,
    PeerId,
};
use libp2p_quic as quic;
use anyhow::Result;
use crate::protocol;
use crate::blob_transfer;

// prepare gossipsub behaviour
fn prepare_gossipsub_behaviour(
    keypair: &identity::Keypair,
)-> Result<gossipsub::Behaviour> {
    // content-address messages
    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };
    // set a custom Gossipsub configuration
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10)) // aid debugging by not cluttering log space
        .validation_mode(gossipsub::ValidationMode::Strict) // enforce message signing
        .message_id_fn(message_id_fn) 
        .build()?;
    Ok(
        gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config
        )
        .map_err(anyhow::Error::msg)?
    )
}

// prepare request-response behaviour
fn prepare_request_response_behaviour()
-> request_response::cbor::Behaviour<protocol::Request, protocol::Response> 
{
    request_response::cbor::Behaviour::<protocol::Request, protocol::Response>::new(
        [(
            StreamProtocol::new("/wholesum/req_resp/1.0"),
            request_response::ProtocolSupport::Full,
        )],
        request_response::Config::default(),
    )
}

// prepare blob-transfer behaviour
fn prepare_blob_transfer_behaviour()
-> request_response::Behaviour::<blob_transfer::BlobCodec> 
{
    request_response::Behaviour::with_codec(
        blob_transfer::BlobCodec,
        [(
            StreamProtocol::new("/wholesum/blob_transfer/1.0"),
            request_response::ProtocolSupport::Full,
        )],
        request_response::Config::default()
            .with_request_timeout(
                Duration::from_secs(60)
            )
    )
}

// prepare identify behaviour
fn prepare_identify_behaviour(
    public_key: &identity::PublicKey
)-> identify::Behaviour {
    identify::Behaviour::new(
        identify::Config::new(
            String::from("/wholesum/identify/1.0"),
            public_key.clone()
        )
    )
}

fn prepare_kademlia_behaviour(
    public_key: &identity::PublicKey,
) -> kad::Behaviour<MemoryStore> {
    let mut cfg = kad::Config::new(
        StreamProtocol::new("/wholesum/kad/1.0")
    );
    cfg.set_query_timeout(Duration::from_secs(5 * 60));    
    let local_peer_id = PeerId::from(public_key.clone());
    let store = MemoryStore::new(local_peer_id);
    kad::Behaviour::with_config(local_peer_id, store, cfg)
}

// main network behaviour 
#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub identify: identify::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub gossipsub: gossipsub::Behaviour,
    pub req_resp: request_response::cbor::Behaviour<protocol::Request, protocol::Response>,
    pub blob_transfer: request_response::Behaviour<blob_transfer::BlobCodec>,
}

// setup a global swram instance
pub fn setup_swarm(
    keypair: &identity::Keypair,
)-> Result<Swarm<MyBehaviour>> {
    let local_keypair = keypair.clone();
    let swarm = SwarmBuilder::with_existing_identity(local_keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default
        )?
        .with_quic()
        .with_dns()?
        .with_behaviour(|key| {            
            let public_key = key.public();
            Ok(MyBehaviour {
                identify: prepare_identify_behaviour(&public_key),
                kademlia: prepare_kademlia_behaviour(&public_key),
                gossipsub: prepare_gossipsub_behaviour(&key)?,
                req_resp: prepare_request_response_behaviour(),
                blob_transfer: prepare_blob_transfer_behaviour()
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();
    Ok(swarm)
}

// used by bootnodes for peer discovery
#[derive(NetworkBehaviour)]
pub struct BootNodeBehaviour {
    pub identify: identify::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

// setup a bootnode-specific swram instance
pub fn setup_swarm_for_bootnode(
    keypair: &identity::Keypair,
)-> Result<Swarm<BootNodeBehaviour>> {
    let local_keypair = keypair.clone();
    let swarm = libp2p::SwarmBuilder::with_existing_identity(local_keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default
        )?
        .with_quic()
        .with_dns()?        
        .with_behaviour(|key| {            
            let public_key = key.public();
            Ok(BootNodeBehaviour {
                identify: prepare_identify_behaviour(&public_key),
                kademlia: prepare_kademlia_behaviour(&public_key)
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();
    Ok(swarm)
}

pub fn prepare_quic_transport(
    keypair: &identity::Keypair
) -> Result<Boxed<(PeerId, StreamMuxerBox)>> {    
    // 1. Create QUIC Config
    let mut quic_config = quic::Config::new(keypair);

    // 2. Tune for 4G / Large Transfers (Optional but recommended)
    // Quinn (the underlying engine) defaults are usually good, but we can 
    // ensure the handshake doesn't timeout on high-latency links.
    quic_config.handshake_timeout = std::time::Duration::from_secs(20);

    // 3. Build the Transport
    // "quic::tokio::Transport" handles the UDP socket internally.
    let transport = quic::tokio::Transport::new(quic_config);

    // 4. Map to Standard Libp2p Types
    // We map the error to a generic IO error to satisfy the Boxed trait constraints
    let transport = transport
        .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        .boxed();

    Ok(transport)
}