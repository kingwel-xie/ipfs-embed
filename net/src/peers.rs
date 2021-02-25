use std::task::{Context, Poll};
use std::time::Duration;
use fnv::{FnvHashMap, FnvHashSet};
use libp2p_rs::core::{Multiaddr, PeerId};


#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PeerInfo {
    protocol_version: Option<String>,
    agent_version: Option<String>,
    protocols: Vec<String>,
    addresses: FnvHashMap<Multiaddr, AddressSource>,
    rtt: Option<Duration>,
}

impl PeerInfo {
    pub fn protocol_version(&self) -> Option<&str> {
        self.protocol_version.as_deref()
    }

    pub fn agent_version(&self) -> Option<&str> {
        self.agent_version.as_deref()
    }

    pub fn protocols(&self) -> impl Iterator<Item = &str> + '_ {
        self.protocols.iter().map(|s| &**s)
    }

    pub fn addresses(&self) -> impl Iterator<Item = (&Multiaddr, AddressSource)> + '_ {
        self.addresses.iter().map(|(addr, source)| (addr, *source))
    }

    pub fn rtt(&self) -> Option<Duration> {
        self.rtt
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressSource {
    Mdns,
    Kad,
    User,
}

#[derive(Debug)]
pub struct AddressBook {
    local_peer_id: PeerId,
    peers: FnvHashMap<PeerId, PeerInfo>,
    connections: FnvHashSet<(PeerId, Multiaddr)>,
}

impl AddressBook {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self {
            local_peer_id,
            peers: Default::default(),
            connections: Default::default(),
        }
    }

    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    pub fn add_address(&mut self, peer: &PeerId, address: Multiaddr, source: AddressSource) {
        if peer == self.local_peer_id() {
            return;
        }
        tracing::trace!(
            "adding address {} for peer {} from {:?}",
            address,
            peer,
            source
        );
        let info = self.peers.entry(*peer).or_default();
        info.addresses.insert(address, source);
    }

    pub fn remove_address(&mut self, peer: &PeerId, address: &Multiaddr) {
        if let Some(info) = self.peers.get_mut(peer) {
            tracing::trace!("removing address {} for peer {}", address, peer);
            info.addresses.remove(address);
        }
    }

    pub fn peers(&self) -> impl Iterator<Item = &PeerId> + '_ {
        self.peers.keys()
    }

    pub fn connections(&self) -> impl Iterator<Item = (&PeerId, &Multiaddr)> + '_ {
        self.connections.iter().map(|(peer, addr)| (peer, addr))
    }

    pub fn info(&self, peer_id: &PeerId) -> Option<&PeerInfo> {
        self.peers.get(peer_id)
    }

    pub fn set_rtt(&mut self, peer_id: &PeerId, rtt: Option<Duration>) {
        if let Some(info) = self.peers.get_mut(peer_id) {
            info.rtt = rtt;
        }
    }

    // pub fn set_info(&mut self, peer_id: &PeerId, identify: IdentifyInfo) {
    //     if let Some(info) = self.peers.get_mut(peer_id) {
    //         info.protocol_version = Some(identify.protocol_version);
    //         info.agent_version = Some(identify.agent_version);
    //         info.protocols = identify.protocols;
    //     }
    // }
}
//
// impl NetworkBehaviour for AddressBook {
//     type ProtocolsHandler = DummyProtocolsHandler;
//     type OutEvent = void::Void;
//
//     fn new_handler(&mut self) -> Self::ProtocolsHandler {
//         Default::default()
//     }
//
//     fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
//         if let Some(info) = self.peers.get(peer_id) {
//             info.addresses().map(|(addr, _)| addr.clone()).collect()
//         } else {
//             vec![]
//         }
//     }
//
//     fn inject_connected(&mut self, _peer_id: &PeerId) {}
//
//     fn inject_disconnected(&mut self, _peer_id: &PeerId) {}
//
//     fn inject_event(&mut self, _peer_id: PeerId, _connection: ConnectionId, _event: void::Void) {}
//
//     fn poll(
//         &mut self,
//         _cx: &mut Context,
//         _params: &mut impl PollParameters,
//     ) -> Poll<NetworkBehaviourAction<void::Void, void::Void>> {
//         Poll::Pending
//     }
//
//     fn inject_connection_established(
//         &mut self,
//         peer_id: &PeerId,
//         _: &ConnectionId,
//         conn: &ConnectedPoint,
//     ) {
//         let conn = (*peer_id, conn.get_remote_address().clone());
//         self.connections.insert(conn);
//     }
//
//     fn inject_address_change(
//         &mut self,
//         peer_id: &PeerId,
//         _: &ConnectionId,
//         old: &ConnectedPoint,
//         new: &ConnectedPoint,
//     ) {
//         let old = (*peer_id, old.get_remote_address().clone());
//         let new = (*peer_id, new.get_remote_address().clone());
//         self.connections.remove(&old);
//         self.connections.insert(new);
//     }
//
//     fn inject_connection_closed(
//         &mut self,
//         peer_id: &PeerId,
//         _: &ConnectionId,
//         conn: &ConnectedPoint,
//     ) {
//         let conn = (*peer_id, conn.get_remote_address().clone());
//         self.connections.remove(&conn);
//     }
//
//     fn inject_addr_reach_failure(
//         &mut self,
//         peer_id: Option<&PeerId>,
//         addr: &Multiaddr,
//         _error: &dyn std::error::Error,
//     ) {
//         if let Some(peer_id) = peer_id {
//             self.remove_address(peer_id, addr);
//         }
//     }
//
//     fn inject_dial_failure(&mut self, peer_id: &PeerId) {
//         self.peers.remove(peer_id);
//     }
// }
