use crate::PacketHandler;
use alloc::vec::Vec;
use secp256k1::{ellswift::ElligatorSwift, SecretKey};

#[derive(Debug)]
pub enum NetworkMagic {
    Mainnet,
    Signet,
}

/// A point on the curve used to complete the handshake.
#[derive(Clone, Debug)]
pub struct EcdhPoint {
    pub(crate) secret_key: SecretKey,
    pub(crate) elligator_swift: ElligatorSwift,
}

/// The result of initiating a handshake.
#[derive(Clone, Debug)]
pub struct InitiatorHandshake {
    /// The message that must be send to the responder.
    pub message: Vec<u8>,
    /// The derived point on the curve used for ECDH.
    pub point: EcdhPoint,
    pub(crate) garbage: Vec<u8>,
}

/// The result of responding to a handshake.
#[derive(Clone, Debug)]
pub struct ResponderHandshake {
    /// The message to send to the initializer.
    pub message: Vec<u8>,
    pub(crate) session_keys: SessionKeyMaterial,
    /// The struct used to encode and decode subsequent packets.
    pub packet_handler: PacketHandler,
    pub(crate) initiator_garbage: Vec<u8>,
}

/// The result after completing a handshake.
pub struct CompleteHandshake {
    /// The final message to send to the responder.
    pub message: Vec<u8>,
    /// The struct used to encode and decode subsequent packets.
    pub packet_handler: PacketHandler,
}

/// All keys derived from the ECDH.
#[derive(Debug, Clone)]
pub struct SessionKeyMaterial {
    /// A unique ID to identify a connection.
    pub session_id: [u8; 32],
    pub(crate) initiator_length_key: [u8; 32],
    pub(crate) initiator_packet_key: [u8; 32],
    pub(crate) responder_length_key: [u8; 32],
    pub(crate) responder_packet_key: [u8; 32],
    pub initiator_garbage_terminator: [u8; 16],
    pub responder_garbage_terminator: [u8; 16],
}

/// Your role in the handshake.
#[derive(Clone, Debug)]
pub enum HandshakeRole {
    /// You started the handshake with a peer.
    Initiator,
    /// You are responding to a handshake.
    Responder,
}

/// A message or decoy packet from a connected peer.
#[derive(Clone, Debug)]
pub struct ReceivedMessage {
    /// A message to handle or `None` if the peer sent a decoy and the message may be safely ignored.
    pub message: Option<Vec<u8>>,
}
