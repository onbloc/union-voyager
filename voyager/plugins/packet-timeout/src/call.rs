use enumorph::Enumorph;
use ibc_union_spec::{ClientId, Timestamp, event::PacketSend};
use macros::model;
use voyager_sdk::primitives::ChainId;

#[model]
#[derive(Enumorph)]
pub enum ModuleCall {
    WaitForTimeoutOrReceipt(WaitForTimeoutOrReceipt),
    MakeMsgTimeout(MakeMsgTimeout),
    UpdateClientToHeightTimestamp(UpdateClientToHeightTimestamp),
    MakeMsgTimeoutFromTrustedHeight(MakeMsgTimeoutFromTrustedHeight),
    CommitProofLensNonMembershipProof(CommitProofLensNonMembershipProof),
}

#[model]
pub struct WaitForTimeoutOrReceipt {
    pub event: PacketSend,
    pub chain_id: ChainId,
    pub counterparty_chain_id: ChainId,
}

#[model]
pub struct MakeMsgTimeout {
    pub event: PacketSend,
    pub chain_id: ChainId,
    pub counterparty_chain_id: ChainId,
}

#[model]
pub struct UpdateClientToHeightTimestamp {
    pub chain_id: ChainId,
    pub counterparty_chain_id: ChainId,
    pub client_id: ClientId,
    pub timestamp: Timestamp,
}

#[model]
pub struct MakeMsgTimeoutFromTrustedHeight {
    pub event: PacketSend,
    pub chain_id: ChainId,
    pub counterparty_chain_id: ChainId,
}

/// Commit a non-membership proof (proving that `event.packet` has not been received on
/// `counterparty_chain_id`) onto the L1 that the proof lens client on `chain_id` is anchored to,
/// so that the lens client can subsequently verify it by proxy.
#[model]
pub struct CommitProofLensNonMembershipProof {
    pub event: PacketSend,
    pub chain_id: ChainId,
    pub counterparty_chain_id: ChainId,
}
