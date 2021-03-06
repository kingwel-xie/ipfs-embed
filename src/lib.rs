//! Ipfs embed is a small, fast and reliable ipfs implementation designed for embedding in to
//! complex p2p applications.
//!
//! ```
//! # #[async_std::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use ipfs_embed::{Config, DefaultParams, Ipfs};
//! # let cache_size = 100;
//! let ipfs = Ipfs::<DefaultParams>::new(Config::new(None, cache_size, "/ip4/0.0.0.0/tcp/0".parse().unwrap())).await?;
//! ipfs.listen_on(?).await?;
//! # Ok(()) }

mod cli;

use async_trait::async_trait;
use futures::channel::mpsc;
use futures::stream::StreamExt;
pub use ipfs_embed_net::{
    Key, Multiaddr, NetworkConfig, PeerId, Record,
};
use ipfs_embed_net::{BitswapStore, NetworkService, Keypair, Subscription, Topic, xcli::App, swarm_cli_commands, dht_cli_commands};
pub use ipfs_embed_sqlite::{StorageConfig, TempPin};
use ipfs_embed_sqlite::{StorageEvent, StorageService};
use libipld::codec::References;
use libipld::error::BlockNotFound;
pub use libipld::store::DefaultParams;
use libipld::store::{Store, StoreParams};
use libipld::{Block, Cid, Ipld, Result};
use prometheus::{Encoder, Registry};
use std::net::SocketAddr;
use std::sync::Arc;
use crate::cli::ipfs_cli_commands;
use futures::Future;

/// Ipfs configuration.
#[derive(Clone, Debug)]
pub struct Config {
    /// Storage configuration.
    pub storage: StorageConfig,
    /// Network configuration.
    pub network: NetworkConfig,
}

impl Config {
    /// Creates a default configuration from a `path` and a `cache_size`. If the `path` is `None`,
    /// ipfs will use an in-memory block store.
    pub fn new(path: Option<std::path::PathBuf>, cache_size: u64, listen_addr: Multiaddr) -> Self {
        let sweep_interval = std::time::Duration::from_millis(10000);
        let storage = StorageConfig::new(path, cache_size, sweep_interval);
        let network = NetworkConfig::new(vec![listen_addr]);
        Self { storage, network }
    }
}

/// Ipfs node.
#[derive(Clone)]
pub struct Ipfs<P: StoreParams> {
    keypair: Keypair,
    storage: StorageService<P>,
    network: NetworkService,
}

#[derive(Clone)]
struct BitswapStorage<P: StoreParams>(StorageService<P>);

impl<P: StoreParams> BitswapStore for BitswapStorage<P>
where
    Ipld: References<P::Codecs>,
{
    type Params = P;

    fn contains(&mut self, cid: &Cid) -> Result<bool> {
        self.0.contains(cid)
    }

    fn get(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>> {
        self.0.get(cid)
    }

    fn insert(&mut self, block: &Block<P>) -> Result<()> {
        self.0.insert(block)
    }

    fn missing_blocks(&mut self, cid: &Cid) -> Result<Vec<Cid>> {
        self.0.missing_blocks(cid)
    }
}

impl<P: StoreParams> Ipfs<P>
where
    Ipld: References<P::Codecs>,
{
    /// Creates a new `Ipfs` from a `Config`.
    ///
    /// This starts three background tasks. The swarm, garbage collector and the dht cleanup
    /// tasks run in the background.
    pub async fn new(config: Config) -> Result<Self> {
        let keypair = config.network.node_key.clone();

        let (tx, mut storage_events) = mpsc::unbounded();
        let storage = StorageService::open(config.storage, tx)?;
        let bitswap = BitswapStorage(storage.clone());
        let network = NetworkService::new(config.network, bitswap).await?;

        let mut _kad = network.kad();
        async_global_executor::spawn(async move {
            while let Some(StorageEvent::Remove(cid)) = storage_events.next().await {
                //let _ = kad.unprovide(cid.to_bytes()).await;
                tracing::debug!("stop providing {} ...", cid);
            }
        })
        .detach();

        Ok(Self { keypair, storage, network })
    }

    /// Returns the local `PeerId`.
    pub fn local_peer_id(&self) -> PeerId {
        self.keypair.public().into_peer_id()
    }

    pub fn run_cli(&self) {
        let mut app = App::new("xCLI");

        app.add_subcommand_with_userdata(swarm_cli_commands(), Box::new(self.network.swarm()));
        app.add_subcommand_with_userdata(dht_cli_commands(), Box::new(self.network.kad()));
        app.add_subcommand_with_userdata(ipfs_cli_commands(), Box::new(self.clone()));
        // app.add_subcommand_with_userdata(bitswap_cli_commands(), Box::new(self.controls.bitswap_mut().clone()));

        app.run();
    }

    // /// Listens on a new `Multiaddr`.
    // pub async fn listen_on(&self, addr: Multiaddr) -> Result<Multiaddr> {
    //     self.network.listen_on(addr).await
    // }

    /// Returns the currently active listener addresses.
    pub async fn listeners(&self) -> Vec<Multiaddr> {
        self.network.swarm().self_addrs().await.unwrap_or_default()
    }

    // /// Adds an external address.
    // pub fn add_external_address(&self, addr: Multiaddr) {
    //     self.network.add_external_address(addr)
    // }
    //
    // /// Returns the currently used external addresses.
    // pub fn external_addresses(&self) -> Vec<AddressRecord> {
    //     self.network.external_addresses()
    // }
    //
    // /// Adds a known `Multiaddr` for a `PeerId`.
    // pub fn add_address(&self, peer: &PeerId, addr: Multiaddr) {
    //     self.network.add_address(peer, addr)
    // }
    //
    // /// Removes a `Multiaddr` for a `PeerId`.
    // pub fn remove_address(&self, peer: &PeerId, addr: &Multiaddr) {
    //     self.network.remove_address(peer, addr)
    // }

    /// Dials a `PeerId` using a known address.
    pub async fn dial(&self, peer: &PeerId) -> Result<()> {
        let _= self.network.swarm().new_connection(*peer).await?;
        Ok(())
    }

    /// Dials a `PeerId` using `Multiaddr`.
    pub async fn dial_address(&self, peer: &PeerId, addr: Multiaddr) -> Result<()> {
        let _= self.network.swarm().connect_with_addrs(*peer, vec![addr]).await?;
        Ok(())
    }

    // /// Bans a `PeerId` from the swarm, dropping all existing connections and
    // /// preventing new connections from the peer.
    // pub fn ban(&self, peer: PeerId) {
    //     self.network.ban(peer)
    // }
    //
    // /// Unbans a previously banned `PeerId`.
    // pub fn unban(&self, peer: PeerId) {
    //     self.network.unban(peer)
    // }

    /// Returns the known peers.
    pub fn peers(&self) -> Vec<PeerId> {
        self.network.swarm().get_peers()
    }

    /// Returns a list of connected peers.
    pub async fn connections(&self) -> Vec<(PeerId, Multiaddr)> {
        let connections = self.network.swarm().dump_connections(None)
            .await
            .unwrap_or_default();

        let cc = connections
            .into_iter()
            .map(|c| {
                (c.info.remote_peer_id, c.info.ra)
            })
            .collect();

        cc
    }

    /// Bootstraps the dht using a set of bootstrap nodes. After bootstrap completes it
    /// provides all blocks in the block store.
    pub async fn bootstrap(&self, nodes: &[(PeerId, Multiaddr)]) -> Result<()> {
        self.network.kad().bootstrap(nodes.to_vec()).await;

        for cid in self.storage.iter()? {
            let _ = self.network.kad().provide(cid.to_bytes()).await;
        }
        Ok(())
    }

    /// Gets a record from the dht.
    pub async fn get_record(&self, key: &[u8]) -> Result<Vec<u8>> {
        let r = self.network.kad().get_value(key.to_vec()).await?;
        Ok(r)
    }

    /// Puts a new record in the dht.
    pub async fn put_record(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let _ = self.network.kad().put_value(key, value).await?;
        Ok(())
    }

    // /// Removes a record from the dht.
    // pub fn remove_record(&self, key: &Key) {
    //     self.network.remove_record(key)
    // }

    /// Subscribes to a `topic` returning a `Stream` of messages. If all `Stream`s for
    /// a topic are dropped it unsubscribes from the `topic`.
    pub async fn subscribe(&self, topic: &str) -> Result<Subscription> {
        let s = self.network.pubsub().subscribe(Topic::new(topic)).await?;
        Ok(s)
    }

    /// Publishes a new message in a `topic`, sending the message to all subscribed peers.
    pub async fn publish(&self, topic: &str, msg: Vec<u8>) -> Result<()> {
        let _ = self.network.pubsub().publish(Topic::new(topic), msg).await;
        Ok(())
    }

    /// Creates a temporary pin in the block store. A temporary pin is not persisted to disk
    /// and is released once it is dropped.
    pub fn create_temp_pin(&self) -> Result<TempPin> {
        self.storage.create_temp_pin()
    }

    /// Adds a new root to a temporary pin.
    pub fn temp_pin(&self, tmp: &TempPin, cid: &Cid) -> Result<()> {
        self.storage.temp_pin(tmp, std::iter::once(*cid))
    }

    /// Returns an `Iterator` of `Cid`s stored in the block store.
    pub fn iter(&self) -> Result<impl Iterator<Item = Cid>> {
        self.storage.iter()
    }

    /// Checks if the block is in the block store.
    pub fn contains(&self, cid: &Cid) -> Result<bool> {
        self.storage.contains(cid)
    }

    /// Returns a block from the block store.
    pub fn get(&self, cid: &Cid) -> Result<Block<P>> {
        if let Some(data) = self.storage.get(cid)? {
            let block = Block::new_unchecked(*cid, data);
            Ok(block)
        } else {
            Err(BlockNotFound(*cid).into())
        }
    }

    /// Either returns a block if it's in the block store or tries to retrieve it from
    /// a peer.
    pub async fn fetch(&self, cid: &Cid) -> Result<Block<P>> {
        if let Some(data) = self.storage.get(cid)? {
            let block = Block::new_unchecked(*cid, data);
            return Ok(block);
        }
        self.network.bitswap().get(*cid).await?;
        if let Some(data) = self.storage.get(cid)? {
            let block = Block::new_unchecked(*cid, data);
            return Ok(block);
        }
        tracing::error!("block evicted too soon. use a temp pin to keep the block around.");
        Err(BlockNotFound(*cid).into())
    }

    /// Inserts a block in to the block store and announces it to peers.
    pub fn insert(&self, block: &Block<P>) -> Result<impl Future<Output = Result<()>> + '_> {
        let cid = *block.cid();
        self.storage.insert(block)?;

        Ok(self.network.bitswap_rd().has_block_rd(cid))
    }

    /// Manually runs garbage collection to completion. This is mainly useful for testing and
    /// administrative interfaces. During normal operation, the garbage collector automatically
    /// runs in the background.
    pub async fn evict(&self) -> Result<()> {
        self.storage.evict().await
    }

    pub async fn sync(&self, cid: &Cid) -> Result<()> {
        let _missing = self.storage.missing_blocks(cid).ok().unwrap_or_default();

        // Currently bitswap doesn't support sync() or get_blocks() method.
        // It requires returning a Stream which returns a Result for every missing Cid
        //self.network.bitswap().sync(*cid, missing);
        Ok(())
    }

    /// Creates, updates or removes an alias with a new root `Cid`.
    pub fn alias<T: AsRef<[u8]> + Send + Sync>(&self, alias: T, cid: Option<&Cid>) -> Result<()> {
        self.storage.alias(alias.as_ref(), cid)
    }

    /// Returns the root of an alias.
    pub fn resolve<T: AsRef<[u8]> + Send + Sync>(&self, alias: T) -> Result<Option<Cid>> {
        self.storage.resolve(alias.as_ref())
    }

    /// Returns a list of aliases preventing a `Cid` from being garbage collected.
    pub fn reverse_alias(&self, cid: &Cid) -> Result<Option<Vec<Vec<u8>>>> {
        self.storage.reverse_alias(cid)
    }

    /// Flushes the block store. After `flush` completes successfully it is guaranteed that
    /// all writes have been persisted to disk.
    pub async fn flush(&self) -> Result<()> {
        self.storage.flush().await
    }

    /// Registers prometheus metrics in a registry.
    pub fn register_metrics(&self, registry: &Registry) -> Result<()> {
        self.storage.register_metrics(registry)?;
        //self.network.register_metrics(registry)?;
        Ok(())
    }
}

/// Telemetry server
pub fn telemetry<P: StoreParams>(addr: SocketAddr, ipfs: &Ipfs<P>) -> Result<()>
where
    Ipld: References<P::Codecs>,
{
    let registry = prometheus::default_registry();
    ipfs.register_metrics(registry)?;
    let mut s = tide::new();
    s.at("/metrics").get(get_metric);
    async_global_executor::spawn(async move { s.listen(addr).await }).detach();
    Ok(())
}

/// Return metrics to prometheus
async fn get_metric(_: tide::Request<()>) -> tide::Result {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];

    encoder.encode(&metric_families, &mut buffer).unwrap();

    let response = tide::Response::builder(200)
        .content_type("text/plain; version=0.0.4")
        .body(tide::Body::from(buffer))
        .build();

    Ok(response)
}

#[async_trait]
impl<P: StoreParams> Store for Ipfs<P>
where
    Ipld: References<P::Codecs>,
{
    type Params = P;
    type TempPin = Arc<TempPin>;

    fn create_temp_pin(&self) -> Result<Self::TempPin> {
        Ok(Arc::new(Ipfs::create_temp_pin(self)?))
    }

    fn temp_pin(&self, tmp: &Self::TempPin, cid: &Cid) -> Result<()> {
        Ipfs::temp_pin(self, tmp, cid)
    }

    fn contains(&self, cid: &Cid) -> Result<bool> {
        Ipfs::contains(self, cid)
    }

    fn get(&self, cid: &Cid) -> Result<Block<P>> {
        Ipfs::get(self, cid)
    }

    fn insert(&self, block: &Block<P>) -> Result<()> {
        let _ = Ipfs::insert(self, block)?;
        Ok(())
    }

    fn alias<T: AsRef<[u8]> + Send + Sync>(&self, alias: T, cid: Option<&Cid>) -> Result<()> {
        Ipfs::alias(self, alias, cid)
    }

    fn resolve<T: AsRef<[u8]> + Send + Sync>(&self, alias: T) -> Result<Option<Cid>> {
        Ipfs::resolve(self, alias)
    }

    fn reverse_alias(&self, cid: &Cid) -> Result<Option<Vec<Vec<u8>>>> {
        Ipfs::reverse_alias(self, cid)
    }

    async fn flush(&self) -> Result<()> {
        Ipfs::flush(self).await
    }

    async fn fetch(&self, cid: &Cid) -> Result<Block<Self::Params>> {
        Ipfs::fetch(self, cid).await
    }

    async fn sync(&self, cid: &Cid) -> Result<()> {
        Ipfs::sync(self, cid).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::join;
    use libipld::cbor::DagCborCodec;
    use libipld::multihash::Code;
    use libipld::raw::RawCodec;
    use libipld::store::DefaultParams;
    use libipld::{alias, ipld};
    use std::time::Duration;

    fn tracing_try_init() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init()
            .ok();
    }

    async fn create_store(enable_mdns: bool) -> Result<Ipfs<DefaultParams>> {
        let sweep_interval = Duration::from_millis(10000);
        let storage = StorageConfig::new(None, 10, sweep_interval);

        let mut network = NetworkConfig::new(vec!["/ip4/127.0.0.1/tcp/0".parse()?]);
        network.enable_mdns = enable_mdns;
        network.allow_non_globals_in_dht = true;

        let ipfs = Ipfs::new(Config { storage, network }).await?;
        Ok(ipfs)
    }

    fn create_block(bytes: &[u8]) -> Result<Block<DefaultParams>> {
        Block::encode(RawCodec, Code::Blake3_256, bytes)
    }

    #[async_std::test]
    async fn test_local_store() -> Result<()> {
        tracing_try_init();
        let store = create_store(false).await?;
        let block = create_block(b"test_local_store")?;
        let tmp = store.create_temp_pin()?;
        store.temp_pin(&tmp, block.cid())?;
        let _ = store.insert(&block)?;
        let block2 = store.get(block.cid())?;
        assert_eq!(block.data(), block2.data());
        Ok(())
    }

    #[async_std::test]
    #[cfg(not(target_os = "macos"))] // mdns doesn't work on macos in github actions
    async fn test_exchange_mdns() -> Result<()> {
        tracing_try_init();
        let store1 = create_store(true).await?;
        let store2 = create_store(true).await?;
        let block = create_block(b"test_exchange_mdns")?;
        let tmp1 = store1.create_temp_pin()?;
        store1.temp_pin(&tmp1, block.cid())?;
        let _ = store1.insert(&block)?;
        store1.flush().await?;
        let tmp2 = store2.create_temp_pin()?;
        store2.temp_pin(&tmp2, block.cid())?;
        let block2 = store2.fetch(block.cid()).await?;
        assert_eq!(block.data(), block2.data());
        Ok(())
    }

    #[async_std::test]
    async fn test_exchange_kad() -> Result<()> {
        tracing_try_init();
        let store = create_store(false).await?;
        let store1 = create_store(false).await?;
        let store2 = create_store(false).await?;

        let addr = store.listeners().await[0].clone();
        let peer_id = store.local_peer_id();
        let nodes = [(peer_id, addr)];

        let b1 = store1.bootstrap(&nodes);
        let b2 = store2.bootstrap(&nodes);
        let (r1, r2) = join!(b1, b2);
        r1.unwrap();
        r2.unwrap();

        let block = create_block(b"test_exchange_kad")?;
        let tmp1 = store1.create_temp_pin()?;
        store1.temp_pin(&tmp1, block.cid())?;
        store1.insert(&block)?.await;
        store1.flush().await?;

        let tmp2 = store2.create_temp_pin()?;
        store2.temp_pin(&tmp2, block.cid())?;
        let block2 = store2.fetch(block.cid()).await?;
        assert_eq!(block.data(), block2.data());
        Ok(())
    }

    #[async_std::test]
    async fn test_provider_not_found() -> Result<()> {
        tracing_try_init();
        let store1 = create_store(true).await?;
        let block = create_block(b"test_provider_not_found")?;
        assert!(store1.fetch(block.cid()).await.is_err());

        Ok(())
    }

    macro_rules! assert_pinned {
        ($store:expr, $block:expr) => {
            assert_eq!(
                $store
                    .reverse_alias($block.cid())
                    .unwrap()
                    .map(|a| !a.is_empty()),
                Some(true)
            );
        };
    }

    macro_rules! assert_unpinned {
        ($store:expr, $block:expr) => {
            assert_eq!(
                $store
                    .reverse_alias($block.cid())
                    .unwrap()
                    .map(|a| !a.is_empty()),
                Some(false)
            );
        };
    }

    fn create_ipld_block(ipld: &Ipld) -> Result<Block<DefaultParams>> {
        Block::encode(DagCborCodec, Code::Blake3_256, ipld)
    }

    #[async_std::test]
    async fn test_sync() -> Result<()> {
        tracing_try_init();
        let local1 = create_store(true).await?;
        let local2 = create_store(true).await?;
        let a1 = create_ipld_block(&ipld!({ "a": 0 }))?;
        let b1 = create_ipld_block(&ipld!({ "b": 0 }))?;
        let c1 = create_ipld_block(&ipld!({ "c": [a1.cid(), b1.cid()] }))?;
        let b2 = create_ipld_block(&ipld!({ "b": 1 }))?;
        let c2 = create_ipld_block(&ipld!({ "c": [a1.cid(), b2.cid()] }))?;
        let x = alias!(x);

        let _ = local1.insert(&a1)?;
        let _ = local1.insert(&b1)?;
        let _ = local1.insert(&c1)?;
        local1.alias(x, Some(c1.cid()))?;
        local1.flush().await?;
        assert_pinned!(&local1, &a1);
        assert_pinned!(&local1, &b1);
        assert_pinned!(&local1, &c1);

        local2.alias(&x, Some(c1.cid()))?;
        local2.sync(c1.cid()).await?;
        local2.flush().await?;
        assert_pinned!(&local2, &a1);
        assert_pinned!(&local2, &b1);
        assert_pinned!(&local2, &c1);

        let _ = local2.insert(&b2)?;
        let _ = local2.insert(&c2)?;
        local2.alias(x, Some(c2.cid()))?;
        local2.flush().await?;
        assert_pinned!(&local2, &a1);
        assert_unpinned!(&local2, &b1);
        assert_unpinned!(&local2, &c1);
        assert_pinned!(&local2, &b2);
        assert_pinned!(&local2, &c2);

        local1.alias(x, Some(c2.cid()))?;
        local1.sync(c2.cid()).await?;
        local1.flush().await?;
        assert_pinned!(&local1, &a1);
        assert_unpinned!(&local1, &b1);
        assert_unpinned!(&local1, &c1);
        assert_pinned!(&local1, &b2);
        assert_pinned!(&local1, &c2);

        local2.alias(x, None)?;
        local2.flush().await?;
        assert_unpinned!(&local2, &a1);
        assert_unpinned!(&local2, &b1);
        assert_unpinned!(&local2, &c1);
        assert_unpinned!(&local2, &b2);
        assert_unpinned!(&local2, &c2);

        local1.alias(x, None)?;
        local2.flush().await?;
        assert_unpinned!(&local1, &a1);
        assert_unpinned!(&local1, &b1);
        assert_unpinned!(&local1, &c1);
        assert_unpinned!(&local1, &b2);
        assert_unpinned!(&local1, &c2);
        Ok(())
    }

    #[async_std::test]
    #[allow(clippy::eval_order_dependence)]
    async fn test_dht_record() -> Result<()> {
        tracing_try_init();
        let stores = [create_store(false).await?, create_store(false).await?];
        stores[0]
            .bootstrap(&[(stores[1].local_peer_id(), stores[1].listeners().await[0].clone())])
            .await?;
        stores[1]
            .bootstrap(&[(stores[0].local_peer_id(), stores[0].listeners().await[0].clone())])
            .await?;
        let key = b"key".to_vec();

        stores[0]
            .put_record(
                key.clone(), b"hello world".to_vec()
            )
            .await?;
        let records = stores[1].get_record(&key).await?;
        assert_eq!(records.len(), 11);
        Ok(())
    }

    #[async_std::test]
    #[allow(clippy::eval_order_dependence)]
    async fn test_gossip() -> Result<()> {
        tracing_try_init();
        let stores = [
            create_store(false).await?,
            create_store(false).await?,
            create_store(false).await?,
            create_store(false).await?,
            create_store(false).await?,
            create_store(false).await?,
        ];
        let mut subscriptions = vec![];
        let topic = "topic";
        for store in &stores {
            for other in &stores {
                if store.local_peer_id() != other.local_peer_id() {
                    store.dial_address(&other.local_peer_id(), other.listeners().await[0].clone()).await?;
                }
            }
            subscriptions.push(store.subscribe(topic).await?);
        }

        async_std::task::sleep(Duration::from_millis(500)).await;

        stores[0].publish(&topic, b"hello world".to_vec()).await.unwrap();

        for subscription in &mut subscriptions[1..] {
            if let Some(msg) = subscription.next().await {
                assert_eq!(msg.data, &b"hello world"[..]);
            }
        }
        Ok(())
    }
}
