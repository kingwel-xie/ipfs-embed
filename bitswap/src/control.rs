use futures::channel::{mpsc, oneshot};
use futures::SinkExt;

use libipld::{Cid, Result};

use libp2p_rs::core::PeerId;

use crate::bitswap::ControlCommand;
use crate::{Priority, Stats};

#[derive(Clone)]
pub struct Control(mpsc::UnboundedSender<ControlCommand>);

impl Control {
    pub(crate) fn new(tx: mpsc::UnboundedSender<ControlCommand>) -> Self {
        Control(tx)
    }

    /// Closes the bitswap main loop.
    pub fn close(&mut self) {
        // simply close the tx, then exit the main loop
        // TODO: wait for the main loop to exit before returning
        self.0.close_channel();
    }

    /// Retrieves the wanted block.
    ///
    /// A user request
    pub async fn get(&mut self, cid: Cid) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0.send(ControlCommand::WantBlock(cid, tx)).await?;
        rx.await?
    }

    /// Announces a new block.
    ///
    /// A user request
    pub async fn has_block(&mut self, cid: Cid) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0.send(ControlCommand::HasBlock(cid, tx)).await?;
        rx.await?
    }

    pub async fn has_block_rd(&self, cid: Cid) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0.clone().send(ControlCommand::HasBlock(cid, tx)).await?;
        rx.await?
    }

    /// Cancels the wanted block.
    ///
    /// A user request
    pub async fn cancel_block(&mut self, cid: Cid) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0.send(ControlCommand::CancelBlock(cid, tx)).await?;
        rx.await?
    }

    /// Returns the wantlist of local if peer is `None`, or the wantlst of the peer specified.
    ///
    /// A user request
    pub async fn wantlist(&mut self, peer: Option<PeerId>) -> Result<Vec<(Cid, Priority)>> {
        let (tx, rx) = oneshot::channel();
        self.0.send(ControlCommand::WantList(peer, tx)).await?;
        rx.await?
    }

    /// Returns the connected peers.
    ///
    /// A user request
    pub async fn peers(&mut self) -> Result<Vec<PeerId>> {
        let (tx, rx) = oneshot::channel();
        self.0.send(ControlCommand::Peers(tx)).await?;
        rx.await?
    }

    /// Returns the bitswap statistics per peer basis.
    ///
    /// A user request
    pub async fn stats(&mut self) -> Result<Stats> {
        let (tx, rx) = oneshot::channel();
        self.0.send(ControlCommand::Stats(tx)).await?;
        rx.await?
    }
}
