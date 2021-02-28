
use crate::Ipfs;


use async_global_executor::block_on;

use std::convert::TryFrom;
use libipld::DefaultParams;
use libipld::multihash::Code;
use libipld::cbor::DagCborCodec;
use libipld::Cid;
use ipfs_embed_net::xcli::*;

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


fn handler(app: &App) -> Ipfs<DefaultParams>
{
    let value_any = app.get_handler(IPFS).expect(IPFS);
    let ipfs = value_any.downcast_ref::<Ipfs<_>>().expect("ipfs").clone();
    ipfs
}

fn cli_get_block(app: &App, args: &[&str]) -> XcliResult {
    let ipfs = handler(app);

    let cid = if args.len() == 1 {
        Cid::try_from(args[0]).map_err(|e| XcliError::BadArgument(e.to_string()))?
    } else {
        return Err(XcliError::MismatchArgument(1, args.len()));
    };

    block_on(async {
        let r = ipfs.fetch(&cid).await;
        match r {
            Ok(data) => println!("{} {:?}", data.cid(), data.data()),
            Err(e) => println!("not found: {:?}", e),
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

    //let ipld_block = libipld::Block::encode(DagCborCodec, Code::Sha2_256, &string).map_err(|e|XcliError::BadArgument(e.to_string()))?;

    block_on(async {
        // let data = ipfs.insert(&ipld_block)?.await;
        // println!("{} {:?}", ipld_block.cid(), data);


    });

    Ok(CmdExeCode::Ok)
}
