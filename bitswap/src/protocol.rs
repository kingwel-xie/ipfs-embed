use std::error::Error;
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::SinkExt;

use libp2p_rs::core::upgrade::UpgradeInfo;
use libp2p_rs::core::{PeerId, ProtocolId};
use libp2p_rs::swarm::connection::Connection;
use libp2p_rs::swarm::Control as SwarmControl;
use libp2p_rs::swarm::protocol_handler::{IProtocolHandler, Notifiee, ProtocolHandler};
use libp2p_rs::swarm::substream::Substream;
use libp2p_rs::traits::{ReadEx, WriteEx};

use crate::ledger::Message;
use crate::{BS_PROTO_ID};
use libipld::Block;
use libipld::store::StoreParams;

const MAX_BUF_SIZE: usize = 524_288;

pub(crate) enum ProtocolEvent<P> {
    NewPeer(PeerId),
    DeadPeer(PeerId),
    Blocks(PeerId, Vec<Block<P>>),
}

#[derive(Clone)]
pub struct Handler<P: StoreParams> {
    incoming_tx: mpsc::UnboundedSender<(PeerId, Message<P>)>,
    new_peer: mpsc::UnboundedSender<ProtocolEvent<P>>,
}

impl<P: StoreParams> Handler<P> {
    pub(crate) fn new(
        incoming_tx: mpsc::UnboundedSender<(PeerId, Message<P>)>,
        new_peer: mpsc::UnboundedSender<ProtocolEvent<P>>,
    ) -> Self {
        Handler {
            incoming_tx,
            new_peer,
        }
    }
}

impl<P: StoreParams + Send> UpgradeInfo for Handler<P> {
    type Info = ProtocolId;

    fn protocol_info(&self) -> Vec<Self::Info> {
        vec![BS_PROTO_ID.into()]
    }
}

impl<P: StoreParams> Notifiee for Handler<P> {
    fn connected(&mut self, conn: &mut Connection) {
        let peer_id = conn.remote_peer();
        let new_peers = self.new_peer.clone();
        let _ = new_peers.unbounded_send(ProtocolEvent::NewPeer(peer_id));
    }
    fn disconnected(&mut self, conn: &mut Connection) {
        let peer_id = conn.remote_peer();
        let new_peers = self.new_peer.clone();
        let _ = new_peers.unbounded_send(ProtocolEvent::DeadPeer(peer_id));
    }
}

#[async_trait]
impl<P: StoreParams + Send> ProtocolHandler for Handler<P> {
    async fn handle(
        &mut self,
        mut stream: Substream,
        _info: <Self as UpgradeInfo>::Info,
    ) -> Result<(), Box<dyn Error>> {
        log::trace!("Handle stream from {}", stream.remote_peer());
        loop {
            let packet = stream.read_one(MAX_BUF_SIZE).await?;
            let message = Message::from_bytes(&packet)?;
            let peer = stream.remote_peer();
            self.incoming_tx.send((peer, message)).await?;
        }
    }

    fn box_clone(&self) -> IProtocolHandler {
        Box::new(self.clone())
    }
}

// Sends bitswap message to remote peer.
pub(crate) async fn send_message<P: StoreParams>(mut swarm: SwarmControl, peer_id: PeerId, message: Message<P>) -> Result<(), Box<dyn Error>> {
    log::debug!("sending message to {:?}...", peer_id);
    let mut stream = swarm.new_stream(peer_id.clone(), vec![BS_PROTO_ID.into()]).await?;
    stream.write_one(message.to_bytes().as_ref()).await?;
    Ok(())
}
