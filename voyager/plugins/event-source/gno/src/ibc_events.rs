use std::{error::Error, str::FromStr};

use ibc_union_spec::{ChannelId, ClientId, ConnectionId, Timestamp};
use serde::{Deserialize, Serialize};
use tracing::warn;
use unionlabs::primitives::{Bytes, H256};
use voyager_sdk::rpc::{RpcError, RpcResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "@type", content = "@value")]
pub enum IbcEvent {
    CreateClient {
        client_id: ClientId,
        client_type: String,
    },

    UpdateClient {
        client_id: ClientId,
        counterparty_height: u64,
    },

    ConnectionOpenInit {
        connection_id: ConnectionId,
        client_id: ClientId,
        counterparty_client_id: ClientId,
    },

    ConnectionOpenTry {
        connection_id: ConnectionId,
        client_id: ClientId,
        counterparty_client_id: ClientId,
        counterparty_connection_id: ConnectionId,
    },

    ConnectionOpenAck {
        connection_id: ConnectionId,
        client_id: ClientId,
        counterparty_client_id: ClientId,
        counterparty_connection_id: ConnectionId,
    },

    ConnectionOpenConfirm {
        connection_id: ConnectionId,
        client_id: ClientId,
        counterparty_client_id: ClientId,
        counterparty_connection_id: ConnectionId,
    },

    ChannelOpenInit(ChannelEvent),

    ChannelOpenTry(ChannelEvent),

    ChannelOpenAck(ChannelEvent),

    ChannelOpenConfirm(ChannelEvent),

    PacketSend {
        packet_hash: H256,
        packet_data: Bytes,
        source_channel_id: ChannelId,
        source_channel_version: String,
        source_connection_id: ConnectionId,
        source_connection_client_id: ClientId,
        destination_channel_id: ChannelId,
        destination_connection_id: ConnectionId,
        destination_connection_client_id: ClientId,
        timeout_timestamp: Timestamp,
    },

    // TODO
    BatchSend {},

    PacketRecv {
        packet_hash: H256,
        packet_data: Bytes,
        source_channel_id: ChannelId,
        source_connection_id: ConnectionId,
        source_connection_client_id: ClientId,
        destination_channel_id: ChannelId,
        destination_channel_version: String,
        destination_connection_id: ConnectionId,
        destination_connection_client_id: ClientId,
        timeout_timestamp: Timestamp,
        maker_msg: Bytes,
    },

    // TODO
    PacketAck {},

    WriteAck {
        packet_hash: H256,
        packet_data: Bytes,
        source_channel_id: ChannelId,
        source_connection_id: ConnectionId,
        source_connection_client_id: ClientId,
        destination_channel_id: ChannelId,
        destination_channel_version: String,
        destination_connection_id: ConnectionId,
        destination_connection_client_id: ClientId,
        timeout_timestamp: Timestamp,
        acknowledgement: Bytes,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChannelEvent {
    pub port_id: String,
    pub channel_id: ChannelId,
    pub counterparty_port_id: Bytes,
    pub counterparty_channel_id: Option<ChannelId>,
    pub connection_id: ConnectionId,
    pub connection_client_id: ClientId,
    pub connection_counterparty_client_id: ClientId,
    pub connection_counterparty_connection_id: ConnectionId,
    pub version: String,
}

impl IbcEvent {
    pub fn is_trivial(&self) -> bool {
        matches!(
            self,
            Self::CreateClient { .. }
                | Self::UpdateClient { .. }
                | Self::PacketRecv { .. }
                // | Self::PacketIntentRecv { .. }
                // | Self::PacketTimeout { .. }
                | Self::PacketAck { .. }
        )
    }

    pub fn from_gno_event(gno_event: gno_rpc::types::event::TmEvent) -> RpcResult<Option<Self>> {
        fn attr<T: FromStr<Err: Error>>(
            attrs: &[gno_rpc::types::EventAttribute],
            ty: &str,
        ) -> RpcResult<T> {
            attrs
                .iter()
                .find_map(|a| (a.key == ty).then(|| a.value.parse()))
                .ok_or_else(|| RpcError::fatal_from_message(format!("key {ty} not found")))?
                .map_err(RpcError::fatal(format!("error parsing value for key {ty}")))
        }

        fn packet_data(attrs: &[gno_rpc::types::EventAttribute]) -> RpcResult<Bytes> {
            if let Ok(packet_data) = attr(attrs, "packet_data") {
                return Ok(packet_data);
            }

            let mut data = String::new();
            for i in 0.. {
                match attrs
                    .iter()
                    .find(|a| a.key == format!("packet_data[{i}]"))
                    .map(|a| &a.value)
                {
                    Some(part) if i == 0 => data.push_str(part),
                    Some(part) => data.push_str(part.strip_prefix("0x").unwrap_or(part)),
                    None if i == 0 => {
                        return Err(RpcError::fatal_from_message("key packet_data not found"));
                    }
                    None => break,
                }
            }

            data.parse()
                .map_err(RpcError::fatal("error parsing value for key packet_data"))
        }

        let attrs = gno_event
            .attrs
            .ok_or_else(|| RpcError::fatal_from_message("no attributes on event"))?;

        let parse_channel_event = |attrs: Vec<gno_rpc::types::EventAttribute>| {
            <RpcResult<_>>::Ok(ChannelEvent {
                port_id: attr(&attrs, "port_id")?,
                channel_id: attr(&attrs, "channel_id")?,
                counterparty_port_id: attr(&attrs, "counterparty_port_id")?,
                // TODO: Fix this once this is no longer emitted on init
                counterparty_channel_id: attr::<ChannelId>(&attrs, "counterparty_channel_id").ok(),
                connection_id: attr(&attrs, "connection_id")?,
                connection_client_id: attr(&attrs, "connection_client_id")?,
                connection_counterparty_client_id: attr(
                    &attrs,
                    "connection_counterparty_client_id",
                )?,
                connection_counterparty_connection_id: attr(
                    &attrs,
                    "connection_counterparty_connection_id",
                )?,
                version: attr(&attrs, "version")?,
            })
        };

        Ok(Some(match &*gno_event.ty {
            "CreateClient" => IbcEvent::CreateClient {
                client_id: attr(&attrs, "client_id")?,
                client_type: attr(&attrs, "client_type")?,
            },
            "UpdateClient" => IbcEvent::UpdateClient {
                client_id: attr(&attrs, "client_id")?,
                counterparty_height: attr(&attrs, "height")?,
            },
            "ConnectionOpenInit" => IbcEvent::ConnectionOpenInit {
                connection_id: attr(&attrs, "connection_id")?,
                client_id: attr(&attrs, "client_id")?,
                counterparty_client_id: attr(&attrs, "counterparty_client_id")?,
            },
            "ConnectionOpenTry" => IbcEvent::ConnectionOpenTry {
                connection_id: attr(&attrs, "connection_id")?,
                client_id: attr(&attrs, "client_id")?,
                counterparty_client_id: attr(&attrs, "counterparty_client_id")?,
                counterparty_connection_id: attr(&attrs, "counterparty_connection_id")?,
            },
            "ConnectionOpenAck" => IbcEvent::ConnectionOpenAck {
                connection_id: attr(&attrs, "connection_id")?,
                client_id: attr(&attrs, "client_id")?,
                counterparty_client_id: attr(&attrs, "counterparty_client_id")?,
                counterparty_connection_id: attr(&attrs, "counterparty_connection_id")?,
            },
            "ConnectionOpenConfirm" => IbcEvent::ConnectionOpenConfirm {
                connection_id: attr(&attrs, "connection_id")?,
                client_id: attr(&attrs, "client_id")?,
                counterparty_client_id: attr(&attrs, "counterparty_client_id")?,
                counterparty_connection_id: attr(&attrs, "counterparty_connection_id")?,
            },
            "ChannelOpenInit" => IbcEvent::ChannelOpenInit(parse_channel_event(attrs)?),
            "ChannelOpenTry" => IbcEvent::ChannelOpenTry(parse_channel_event(attrs)?),
            "ChannelOpenAck" => IbcEvent::ChannelOpenAck(parse_channel_event(attrs)?),
            "ChannelOpenConfirm" => IbcEvent::ChannelOpenConfirm(parse_channel_event(attrs)?),
            "PacketRecv" => IbcEvent::PacketRecv {
                packet_hash: attr(&attrs, "packet_hash")?,
                packet_data: packet_data(&attrs)?,
                source_channel_id: attr(&attrs, "source_channel_id")?,
                source_connection_id: attr(&attrs, "source_connection_id")?,
                source_connection_client_id: attr(&attrs, "source_connection_client_id")?,
                destination_channel_id: attr(&attrs, "destination_channel_id")?,
                destination_channel_version: attr(&attrs, "destination_channel_version")?,
                destination_connection_id: attr(&attrs, "destination_connection_id")?,
                destination_connection_client_id: attr(&attrs, "destination_connection_client_id")?,
                timeout_timestamp: attr(&attrs, "timeout_timestamp")?,
                maker_msg: attr(&attrs, "maker_msg")?,
            },
            "PacketSend" => IbcEvent::PacketSend {
                packet_hash: attr(&attrs, "packet_hash")?,
                packet_data: packet_data(&attrs)?,
                source_channel_id: attr(&attrs, "source_channel_id")?,
                source_channel_version: attr(&attrs, "source_channel_version")?,
                source_connection_id: attr(&attrs, "source_connection_id")?,
                source_connection_client_id: attr(&attrs, "source_connection_client_id")?,
                destination_channel_id: attr(&attrs, "destination_channel_id")?,
                destination_connection_id: attr(&attrs, "destination_connection_id")?,
                destination_connection_client_id: attr(&attrs, "destination_connection_client_id")?,
                timeout_timestamp: attr(&attrs, "timeout_timestamp")?,
            },
            // TODO
            "BatchSend" => IbcEvent::BatchSend {},
            "PacketAck" => IbcEvent::PacketAck {},
            "WriteAck" => IbcEvent::WriteAck {
                packet_hash: attr(&attrs, "packet_hash")?,
                packet_data: packet_data(&attrs)?,
                source_channel_id: attr(&attrs, "source_channel_id")?,
                source_connection_id: attr(&attrs, "source_connection_id")?,
                source_connection_client_id: attr(&attrs, "source_connection_client_id")?,
                destination_channel_id: attr(&attrs, "destination_channel_id")?,
                destination_channel_version: attr(&attrs, "destination_channel_version")?,
                destination_connection_id: attr(&attrs, "destination_connection_id")?,
                destination_connection_client_id: attr(&attrs, "destination_connection_client_id")?,
                timeout_timestamp: attr(&attrs, "timeout_timestamp")?,
                acknowledgement: attr(&attrs, "acknowledgement")?,
            },
            event => {
                warn!("unknown event: {event}");
                return Ok(None);
            }
        }))
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            IbcEvent::CreateClient { .. } => "create_client",
            IbcEvent::UpdateClient { .. } => "update_client",
            IbcEvent::ConnectionOpenInit { .. } => "connection_open_init",
            IbcEvent::ConnectionOpenTry { .. } => "connection_open_try",
            IbcEvent::ConnectionOpenAck { .. } => "connection_open_ack",
            IbcEvent::ConnectionOpenConfirm { .. } => "connection_open_confirm",
            IbcEvent::ChannelOpenInit { .. } => "channel_open_init",
            IbcEvent::ChannelOpenTry { .. } => "channel_open_try",
            IbcEvent::ChannelOpenAck { .. } => "channel_open_ack",
            IbcEvent::ChannelOpenConfirm(ChannelEvent { .. }) => "channel_open_confirm",
            IbcEvent::PacketRecv { .. } => "recv_packet",
            IbcEvent::PacketSend { .. } => "send_packet",
            IbcEvent::BatchSend { .. } => "batch_send",
            IbcEvent::PacketAck { .. } => "acknowledge_packet",
            IbcEvent::WriteAck { .. } => "write_ack",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IbcEvent;
    use gno_rpc::types::{EventAttribute, event::TmEvent};

    fn packet_send(attrs: &[(&str, &str)]) -> IbcEvent {
        IbcEvent::from_gno_event(TmEvent {
            ty: "PacketSend".into(),
            pkg_path: "gno.land/r/onbloc/ibc/union/core".into(),
            attrs: Some(
                [
                    (
                        "packet_hash",
                        "0x0000000000000000000000000000000000000000000000000000000000000000",
                    ),
                    ("source_channel_id", "1"),
                    ("source_channel_version", "ucs03-zkgm-0"),
                    ("source_connection_id", "1"),
                    ("source_connection_client_id", "1"),
                    ("destination_channel_id", "1"),
                    ("destination_connection_id", "1"),
                    ("destination_connection_client_id", "1"),
                    ("timeout_timestamp", "1"),
                ]
                .into_iter()
                .chain(attrs.iter().copied())
                .map(|(key, value)| EventAttribute {
                    key: key.into(),
                    value: value.into(),
                })
                .collect(),
            ),
        })
        .unwrap()
        .unwrap()
    }

    #[test]
    fn parses_packet_send_single_packet_data_attr() {
        let IbcEvent::PacketSend { packet_data, .. } = packet_send(&[("packet_data", "0x0102")])
        else {
            panic!("expected PacketSend");
        };

        assert_eq!(packet_data.to_string(), "0x0102");
    }

    #[test]
    fn parses_packet_send_chunked_packet_data_attrs() {
        let IbcEvent::PacketSend { packet_data, .. } =
            packet_send(&[("packet_data[0]", "0x0102"), ("packet_data[1]", "0x0304")])
        else {
            panic!("expected PacketSend");
        };

        assert_eq!(packet_data.to_string(), "0x01020304");
    }
}
