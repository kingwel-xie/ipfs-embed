use std::num::NonZeroU16;
use std::time::Duration;
use libp2p_rs::core::identity::Keypair;
use libp2p_rs::core::{PublicKey, PeerId, Multiaddr};

/// Network configuration.
#[derive(Clone)]
pub struct NetworkConfig {
    /// Node identity keypair.
    pub node_key: Keypair,
    /// Name of the node. Sent over the wire for debugging purposes.
    pub node_name: String,
    /// Bound listening addresses; by default the node will not listen on any address.
    pub listening_addrs: Vec<Multiaddr>,
    /// The peers to connect to on startup.
    pub bootstrap: Vec<(PeerId, Multiaddr)>,
    /// Enable mdns.
    pub enable_mdns: bool,
    /// Enable kad.
    pub enable_kad: bool,
    /// Should we insert non-global addresses into the DHT?
    pub allow_non_globals_in_dht: bool,
    /// Bitswap request timeout.
    pub bitswap_request_timeout: Duration,
    /// Bitswap connection keep alive.
    pub bitswap_connection_keepalive: Duration,
    /// Bitswap inbound requests per peer limit.
    pub bitswap_receive_limit: NonZeroU16,
    // /// Pre shared key for pnet.
    // pub psk: Option<PreSharedKey>,
}

impl NetworkConfig {
    /// Creates a new network configuration.
    pub fn new(listening_addrs: Vec<Multiaddr>) -> Self {
        Self {
            enable_mdns: true,
            enable_kad: true,
            allow_non_globals_in_dht: false,
            node_key: Keypair::generate_ed25519(),
            node_name: names::Generator::with_naming(names::Name::Numbered)
                .next()
                .unwrap(),
            listening_addrs,
            bitswap_request_timeout: Duration::from_secs(10),
            bitswap_connection_keepalive: Duration::from_secs(10),
            bitswap_receive_limit: NonZeroU16::new(20).expect("20 > 0"),
            //psk: None,
            bootstrap: vec![]
        }
    }

    /// The public node key.
    pub fn public(&self) -> PublicKey {
        self.node_key.public()
    }

    /// The peer id of the node.
    pub fn peer_id(&self) -> PeerId {
        self.node_key.public().into_peer_id()
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl std::fmt::Debug for NetworkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("NetworkConfig")
            .field("node_key", &self.peer_id().to_string())
            .field("node_name", &self.node_name)
            .field("enable_mdns", &self.enable_mdns)
            .field("enable_kad", &self.enable_kad)
            .field("allow_non_globals_in_dht", &self.allow_non_globals_in_dht)
            .field("bitswap_request_timeout", &self.bitswap_request_timeout)
            .field(
                "bitswap_connection_keepalive",
                &self.bitswap_connection_keepalive,
            )
            .field("bitswap_receive_limit", &self.bitswap_receive_limit)
            //.field("psk", &self.psk.is_some())
            .finish()
    }
}
