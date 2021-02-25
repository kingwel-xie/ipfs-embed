use futures::stream::Stream;
use futures::{future, pin_mut};
use libipld::store::StoreParams;
use libipld::{Cid, Result};
use prometheus::Registry;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

mod config;
mod peers;

pub use libp2p_rs::core::identity::Keypair;
pub use libp2p_rs::core::{Multiaddr, PeerId, ProtocolId};
pub use libp2p_rs::kad::record::{Key, Record};

use libp2p_rs::swarm::{Control as SwarmControl, Swarm};
use libp2p_rs::kad::Control as KadControl;
//use libp2p_rs::mdns::control::Control as MdnsControl;
use libp2p_rs::floodsub::control::Control as FloodsubControl;
//use bitswap::Control as BitswapControl;

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

//use bitswap::Bitswap;

pub use crate::config::NetworkConfig;
pub use crate::peers::{AddressSource, PeerInfo};
pub use bitswap::BitswapStore;


#[derive(Clone)]
pub struct NetworkService<P: StoreParams> {
    //repo: Repo<Types>,

    swarm: SwarmControl,
    kad: KadControl,
    pubsub: FloodsubControl,
    // mdns: MdnsControl,
    //bitswap: BitswapControl,
}

impl<P: StoreParams> NetworkService<P> {
    pub async fn new<S: BitswapStore<Params = P>>(config: NetworkConfig, store: S) -> Result<Self> {
        let sec_secio = secio::Config::new(config.node_key.clone());
        // Set up an encrypted TCP transport over the Yamux or Mplex protocol.
        let xx_keypair = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&config.node_key)
            .unwrap();
        let sec_noise = noise::NoiseConfig::xx(xx_keypair, config.node_key.clone());
        let sec = Selector::new(sec_noise, sec_secio);

        // FIXME: timeout & DnsConfig
        let mux = Selector::new(yamux::Config::new(), mplex::Config::new());
        let tu = TransportUpgrade::new(TcpConfig::new().nodelay(true), mux, sec);//.timeout(Duration::from_secs(20));

        // Make swarm
        let mut swarm = Swarm::new(config.node_key.public())
            .with_transport(Box::new(tu))
            .with_ping(PingConfig::new())
            .with_identify(IdentifyConfig::new(false));

        swarm.listen_on(config.listening_addrs)?;

        let swarm_control = swarm.control();

        log::info!("Swarm created, local-peer-id={:?}", swarm.local_peer_id());

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

        // // bitswap
        // let bitswap = Bitswap::new(repo.clone(), kad_control.clone());
        // let bitswap_control = bitswap.control();
        //
        // // register bitswap into Swarm
        // swarm = swarm.with_protocol(bitswap);

        // To start Swarm/Kad/... main loops
        swarm.start();

        // // handle bootstrap nodes
        // for (addr, peer_id) in options.bootstrap {
        //     kad_control.add_node(peer_id, vec![addr]).await;
        // }
        kad_control.bootstrap().await;

        Ok(NetworkService {
            swarm: swarm_control,
            kad: kad_control,
            pubsub: floodsub_control,
            //bitswap: bitswap_control,
            //mdns: ()
        })
    }

    pub fn swarm(&self) -> SwarmControl { self.swarm.clone() }
    pub fn kad(&self) -> KadControl { self.kad.clone() }
    pub fn pubsub(&self) -> FloodsubControl { self.pubsub.clone() }
    //pub fn bitswap(&self) -> BitswapControl { self.bitswap.clone() }

    // pub fn listeners(&self) -> Vec<Multiaddr> {
    //     let swarm = self.swarm.lock().unwrap();
    //     Swarm::listeners(&swarm).cloned().collect()
    // }
    //
    // pub fn add_external_address(&self, addr: Multiaddr) {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     Swarm::add_external_address(&mut swarm, addr, AddressScore::Infinite);
    // }
    //
    // pub fn external_addresses(&self) -> Vec<AddressRecord> {
    //     let swarm = self.swarm.lock().unwrap();
    //     Swarm::external_addresses(&swarm).cloned().collect()
    // }
    //
    // pub fn add_address(&self, peer: &PeerId, addr: Multiaddr) {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     swarm.add_address(peer, addr, AddressSource::User);
    // }
    //
    // pub fn remove_address(&self, peer: &PeerId, addr: &Multiaddr) {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     swarm.remove_address(peer, addr);
    // }
    //
    // pub fn dial(&self, peer: &PeerId) -> Result<()> {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     Ok(Swarm::dial(&mut swarm, peer)?)
    // }
    //
    // pub fn ban(&self, peer: PeerId) {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     Swarm::ban_peer_id(&mut swarm, peer)
    // }
    //
    // pub fn unban(&self, peer: PeerId) {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     Swarm::unban_peer_id(&mut swarm, peer)
    // }
    //
    // pub fn peers(&self) -> Vec<PeerId> {
    //     let swarm = self.swarm.lock().unwrap();
    //     swarm.peers().copied().collect()
    // }
    //
    // pub fn connections(&self) -> Vec<(PeerId, Multiaddr)> {
    //     let swarm = self.swarm.lock().unwrap();
    //     swarm
    //         .connections()
    //         .map(|(peer_id, addr)| (*peer_id, addr.clone()))
    //         .collect()
    // }
    //
    // pub fn peer_info(&self, peer: &PeerId) -> Option<PeerInfo> {
    //     let swarm = self.swarm.lock().unwrap();
    //     swarm.info(peer).cloned()
    // }
    //
    // pub async fn bootstrap(&self, peers: &[(PeerId, Multiaddr)]) -> Result<()> {
    //     for (peer, addr) in peers {
    //         self.add_address(peer, addr.clone());
    //         self.dial(peer)?;
    //     }
    //     let rx = {
    //         let mut swarm = self.swarm.lock().unwrap();
    //         swarm.bootstrap()
    //     };
    //     tracing::trace!("started bootstrap");
    //     rx.await??;
    //     tracing::trace!("boostrap complete");
    //     Ok(())
    // }
    //
    // pub async fn get_record(&self, key: &Key, quorum: Quorum) -> Result<Vec<PeerRecord>> {
    //     let rx = {
    //         let mut swarm = self.swarm.lock().unwrap();
    //         swarm.get_record(key, quorum)
    //     };
    //     Ok(rx.await??)
    // }
    //
    // pub async fn put_record(&self, record: Record, quorum: Quorum) -> Result<()> {
    //     let rx = {
    //         let mut swarm = self.swarm.lock().unwrap();
    //         swarm.put_record(record, quorum)
    //     };
    //     rx.await??;
    //     Ok(())
    // }
    //
    // pub fn subscribe(&self, topic: &str) -> Result<impl Stream<Item = Vec<u8>>> {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     swarm.subscribe(topic)
    // }
    //
    // pub fn publish(&self, topic: &str, msg: Vec<u8>) -> Result<()> {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     swarm.publish(topic, msg)
    // }
    //
    // pub fn remove_record(&self, key: &Key) {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     swarm.remove_record(key)
    // }
    //
    // pub fn get(&self, cid: Cid) -> GetQuery<P> {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     let (rx, id) = swarm.get(cid);
    //     GetQuery {
    //         swarm: Some(self.swarm.clone()),
    //         id,
    //         rx,
    //     }
    // }
    //
    // pub fn sync(&self, cid: Cid, missing: impl Iterator<Item = Cid>) -> SyncQuery<P> {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     let (rx, id) = swarm.sync(cid, missing);
    //     SyncQuery {
    //         swarm: Some(self.swarm.clone()),
    //         id,
    //         rx,
    //     }
    // }
    //
    // pub async fn provide(&self, cid: Cid) -> Result<()> {
    //     let rx = {
    //         let mut swarm = self.swarm.lock().unwrap();
    //         swarm.provide(cid)
    //     };
    //     rx.await??;
    //     Ok(())
    // }
    //
    // pub fn unprovide(&self, cid: Cid) {
    //     let mut swarm = self.swarm.lock().unwrap();
    //     swarm.unprovide(cid)
    // }
    //
    // pub fn register_metrics(&self, registry: &Registry) -> Result<()> {
    //     let swarm = self.swarm.lock().unwrap();
    //     swarm.register_metrics(registry)
    // }
}
