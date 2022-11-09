use crate::event_handler::*;
use crate::ws_connection::WebSocketConnect;
use flowy_derive::{Flowy_Event, ProtoBuf_Enum};
use lib_dispatch::prelude::*;
use std::sync::Arc;
use strum_macros::Display;

pub fn create(ws_conn: Arc<WebSocketConnect>) -> Module {
    Module::new()
        .name("Flowy-Network")
        .data(ws_conn)
        .event(NetworkEvent::UpdateNetworkType, update_network_type)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Display, Hash, ProtoBuf_Enum, Flowy_Event)]
#[event_err = "FlowyError"]
pub enum NetworkEvent {
    #[event(input = "NetworkState")]
    UpdateNetworkType = 0,
}
