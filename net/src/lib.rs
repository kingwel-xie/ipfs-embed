use libipld::Result;
//use prometheus::Registry;
use std::time::Duration;

mod config;

pub use libp2p_rs::core::identity::Keypair;
pub use libp2p_rs::core::{Multiaddr, PeerId, ProtocolId};
pub use libp2p_rs::kad::record::{Key, Record};
pub use libp2p_rs::floodsub::subscription::Subscription;
pub use libp2p_rs::floodsub::Topic;
pub use libp2p_rs::xcli;
pub use libp2p_rs::swarm::cli::swarm_cli_commands;
pub use libp2p_rs::kad::cli::dht_cli_commands;

use libp2p_rs::swarm::{Control as SwarmControl, Swarm};
use libp2p_rs::kad::Control as KadControl;
//use libp2p_rs::mdns::control::Control as MdnsControl;
use libp2p_rs::floodsub::control::Control as FloodsubControl;
use bitswap::Control as BitswapControl;

use libp2p_rs::kad::store::MemoryStore;
use libp2p_rs::swarm::identify::IdentifyConfig;
use libp2p_rs::swarm::ping::PingConfig;
use libp2p_rs::kad::kad::{KademliaConfig, Kademlia};
use libp2p_rs::floodsub::FloodsubConfig;
use libp2p_rs::floodsub::floodsub::FloodSub;
use libp2p_rs::{noise, yamux, mplex, secio};
use libp2p_rs::core::upgrade::Selector;
use libp2p_rs::core::transport::upgrade::TransportUpgrade;
use libp2p_rs::tcp::TcpConfig;

use bitswap::Bitswap;

pub use crate::config::NetworkConfig;
pub use bitswap::{BitswapStore};
use libp2p_rs::dns::DnsConfig;
use libp2p_rs::core::Transport;


#[derive(Clone)]
pub struct NetworkService {
    swarm: SwarmControl,
    kad: KadControl,
    pubsub: FloodsubControl,
    // mdns: MdnsControl,
    bitswap: BitswapControl,
}

impl NetworkService {
    pub async fn new<S: BitswapStore>(config: NetworkConfig, repo: S) -> Result<Self> {
        let sec_secio = secio::Config::new(config.node_key.clone());
        // Set up an encrypted TCP transport over the Yamux or Mplex protocol.
        let xx_keypair = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&config.node_key)
            .unwrap();
        let sec_noise = noise::NoiseConfig::xx(xx_keypair, config.node_key.clone());
        let sec = Selector::new(sec_noise, sec_secio);

        let mux = Selector::new(yamux::Config::new(), mplex::Config::new());
        let transport = TcpConfig::new().nodelay(true).outbound_timeout(Duration::from_secs(10));
        let tu = TransportUpgrade::new(DnsConfig::new(transport), mux, sec);

        // Make swarm
        let mut swarm = Swarm::new(config.node_key.public())
            .with_transport(Box::new(tu))
            .with_ping(PingConfig::new())
            .with_identify(IdentifyConfig::new(false));

        swarm.listen_on(config.listening_addrs)?;

        let swarm_control = swarm.control();

        tracing::info!("Swarm created, local-peer-id={:?}", swarm.local_peer_id());

        // build Kad
        let kad_config = KademliaConfig::default().with_query_timeout(Duration::from_secs(90));

        let store = MemoryStore::new(swarm.local_peer_id().clone());
        let kad = Kademlia::with_config(swarm.local_peer_id().clone(), store, kad_config);

        let mut kad_control = kad.control();

        // update Swarm to support Kad and Routing
        swarm = swarm.with_protocol(kad).with_routing(Box::new(kad_control.clone()));

        let mut floodsub_config = FloodsubConfig::new(swarm.local_peer_id().clone());
        floodsub_config.subscribe_local_messages = true;

        let floodsub = FloodSub::new(floodsub_config);
        let floodsub_control = floodsub.control();

        // register floodsub into Swarm
        swarm = swarm.with_protocol(floodsub);

        // bitswap
        let bitswap = Bitswap::new(repo, kad_control.clone());
        let bitswap_control = bitswap.control();

        // register bitswap into Swarm
        swarm = swarm.with_protocol(bitswap);

        // To start Swarm/Kad/... main loops
        swarm.start();

        // handle bootstrap nodes
        if !config.bootstrap.is_empty() {
            kad_control.bootstrap(config.bootstrap.clone()).await;
        }

        Ok(NetworkService {
            swarm: swarm_control,
            kad: kad_control,
            pubsub: floodsub_control,
            bitswap: bitswap_control,
            //mdns: ()
        })
    }

    pub fn swarm(&self) -> SwarmControl { self.swarm.clone() }
    pub fn kad_mut(&mut self) -> &mut KadControl { &mut self.kad }
    pub fn kad(&self) -> KadControl { self.kad.clone() }
    pub fn pubsub(&self) -> FloodsubControl { self.pubsub.clone() }
    pub fn bitswap(&self) -> BitswapControl { self.bitswap.clone() }

    pub fn bitswap_rd(&self) -> &BitswapControl { &self.bitswap }

    //
    // pub fn register_metrics(&self, registry: &Registry) -> Result<()> {
    //     let swarm = self.swarm.lock().unwrap();
    //     swarm.register_metrics(registry)
    // }
}
