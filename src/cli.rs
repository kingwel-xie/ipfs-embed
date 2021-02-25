
use async_std::task;
use xcli::*;
use crate::Ipfs;

const IPFS: &str = "ipfs";



pub fn ipfs_cli_commands<'a>() -> Command<'a> {
    let get_block_cmd = Command::new("get")
        .about("get block")
        .usage("get <cid>")
        .action(cli_get_block);
    let put_block_cmd = Command::new("put")
        .about("put block")
        .usage("put <string>")
        .action(cli_put_block);

    Command::new_with_alias(IPFS, "i")
        .about("IPFS")
        .usage("ipfs")
        .subcommand(get_block_cmd)
        .subcommand(put_block_cmd)
}

use ipfs_embed_core::{
    AddressRecord, BitswapStorage, BitswapSync, Block, Cid, Multiaddr, Network, NetworkEvent,
    PeerId, Query, QueryResult, QueryType, Result, Storage, StorageEvent, StoreParams,
};
use libipld::codec::References;
use libipld::ipld::Ipld;
use std::convert::TryFrom;
use ipfs_embed_core::Store;
use libipld::DefaultParams;
use ipfs_embed_net::NetworkService;
use std::sync::Arc;
use ipfs_embed_db::StorageService;
use libipld::multihash::Code;
use libipld::cbor::DagCborCodec;

fn handler(app: &App) -> Ipfs<DefaultParams, StorageService<DefaultParams>, NetworkService<DefaultParams>>
{
    let value_any = app.get_handler(IPFS).expect(IPFS);
    let ipfs = value_any.downcast_ref::<Ipfs<_, _, _>>().expect("ipfs").clone();
    ipfs
}

fn cli_get_block(app: &App, args: &[&str]) -> XcliResult {
    let ipfs = handler(app);

    let cid = if args.len() == 1 {
        Cid::try_from(args[0]).map_err(|e| XcliError::BadArgument(e.to_string()))?
    } else {
        return Err(XcliError::MismatchArgument(1, args.len()));
    };

    task::block_on(async {
        let r = ipfs.get(&cid).await;
        if let Ok(data) = r {
            println!("{} {:?}", data.cid(), data.data());
        }

    });

    Ok(CmdExeCode::Ok)
}

fn cli_put_block(app: &App, args: &[&str]) -> XcliResult {
    let ipfs = handler(app);

    let string = if args.len() == 1 {
        String::try_from(args[0]).map_err(|e| XcliError::BadArgument(e.to_string()))?
    } else {
        return Err(XcliError::MismatchArgument(1, args.len()));
    };

    let ipld_block = libipld::Block::encode(DagCborCodec, Code::Sha2_256, &string).map_err(|e|XcliError::BadArgument(e.to_string()))?;

    task::block_on(async {
        let data = ipfs.insert(&ipld_block).await;
        println!("{} {:?}", ipld_block.cid(), data);
    });

    Ok(CmdExeCode::Ok)
}
