use ipfs_embed::{Config, DefaultParams, Ipfs};
use libipld::{DagCbor, Block};
use libipld::store::Store;
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
    let cache_size = 10;
    let ipfs = Ipfs::<DefaultParams>::new(Config::new(None, cache_size)).await?;
    ipfs.listen_on("/ip4/0.0.0.0/tcp/0".parse()?).await?;

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

    Ok(())
}