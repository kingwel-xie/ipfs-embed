
use libipld::{Cid, Block, Result};
use libipld::store::StoreParams;



//
// /// An Ipfs block consisting of a [`Cid`] and the bytes of the block.
// ///
// /// Note: At the moment the equality is based on [`Cid`] equality, which is based on the triple
// /// `(cid::Version, cid::Codec, multihash)`.
// #[derive(Clone, Debug)]
// pub struct Block {
//     /// The content identifier for this block
//     pub cid: Cid,
//     /// The data of this block
//     pub data: Box<[u8]>,
// }
//
// impl PartialEq for Block {
//     fn eq(&self, other: &Self) -> bool {
//         self.cid.hash() == other.cid.hash()
//     }
// }
//
// impl Eq for Block {}
//
// impl Block {
//     pub fn new(data: Box<[u8]>, cid: Cid) -> Self {
//         Self { cid, data }
//     }
//
//     pub fn cid(&self) -> &Cid {
//         &self.cid
//     }
//
//     pub fn data(&self) -> &[u8] {
//         &self.data
//     }
//
//     pub fn into_vec(self) -> Vec<u8> {
//         self.data.into()
//     }
// }
//
// /// BlockStore TRait used by Bitswap.
// #[async_trait]
// pub trait BsBlockStore: Clone + Send + Sync + Unpin + 'static {
//     /// Returns whether a block is present in the blockstore.
//     async fn contains(&self, cid: &Cid) -> Result<bool, Box<dyn Error>>;
//     /// Returns a block from the blockstore.
//     async fn get(&self, cid: &Cid) -> Result<Option<Block>, Box<dyn Error>>;
//     /// Inserts a block in the blockstore. bool means it is a new block inserted,
//     /// otherwise, existing block.
//     async fn put(&self, block: Block) -> Result<(Cid, bool), Box<dyn Error>>;
//     /// Removes a block from the blockstore.
//     async fn remove(&self, cid: &Cid) -> Result<(), Box<dyn Error>>;
// }

/// Trait implemented by a block store.
pub trait BitswapStore: Clone + Send + Sync + 'static {
    /// The store params.
    type Params: StoreParams;
    /// A have query needs to know if the block store contains the block.
    fn contains(&mut self, cid: &Cid) -> Result<bool>;
    /// A block query needs to retrieve the block from the store.
    fn get(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>>;
    /// A block response needs to insert the block into the store.
    fn insert(&mut self, block: &Block<Self::Params>) -> Result<()>;
    /// A sync query needs a list of missing blocks to make progress.
    fn missing_blocks(&mut self, cid: &Cid) -> Result<Vec<Cid>>;
}