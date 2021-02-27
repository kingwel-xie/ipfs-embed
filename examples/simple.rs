use ipfs_embed::{Config, DefaultParams, Ipfs};

use libipld::{DagCbor, Block};
use libipld::cbor::DagCborCodec;
use libipld::multihash::Code;

#[derive(Clone, DagCbor, Debug, Eq, PartialEq)]
struct Identity {
    id: u64,
    name: String,
    age: u8,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        //.with_max_level(tracing::Level::DEBUG)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cache_size = 10;
    let mut config = Config::new(None, cache_size, "/ip4/0.0.0.0/tcp/8082".parse()?);
    config.network.bootstrap.push(("/ip4/104.131.131.82/tcp/4001".parse().unwrap(), "QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ".parse().unwrap()));

    let ipfs = Ipfs::<DefaultParams>::new(config).await?;

    let identity = Identity {
        id: 0,
        name: "David Craven".into(),
        age: 26,
    };

    let ipld_block = libipld::Block::encode(DagCborCodec, Code::Blake3_256, &identity)?;
    let cid = *ipld_block.cid();

    let _ = ipfs.insert(&ipld_block)?;
    let identity2 = ipfs.get(&cid)?;
    assert_eq!(ipld_block, identity2);
    println!("identity cid is {}", cid);

    ipfs.run_cli();

    Ok(())
}