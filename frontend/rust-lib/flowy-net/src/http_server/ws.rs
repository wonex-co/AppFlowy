use crate::ws_connection::{RawWebSocket, WebSocketMessageSender};
use flowy_error::internal_error;
pub use flowy_error::FlowyError;
use futures_util::future::BoxFuture;
use lib_infra::future::FutureResult;
pub use lib_ws::{WSConnectState, WSMessageReceiver, WebSocketRawMessage};
use lib_ws::{WSController, WSSender};
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

pub type HttpWebSocket = Arc<WSController>;

impl RawWebSocket for HttpWebSocket {
    fn initialize(&self) -> FutureResult<(), FlowyError> {
        FutureResult::new(async { Ok(()) })
    }

    fn start_connect(&self, addr: String, _user_id: String) -> FutureResult<(), FlowyError> {
        let cloned_ws = self.clone();
        FutureResult::new(async move {
            let _ = cloned_ws.start(addr).await.map_err(internal_error)?;
            Ok(())
        })
    }

    fn stop_connect(&self) -> FutureResult<(), FlowyError> {
        let controller = self.clone();
        FutureResult::new(async move {
            controller.stop().await;
            Ok(())
        })
    }

    fn subscribe_connect_state(&self) -> BoxFuture<Receiver<WSConnectState>> {
        let cloned_ws = self.clone();
        Box::pin(async move { cloned_ws.subscribe_state().await })
    }

    fn reconnect(&self, count: usize) -> FutureResult<(), FlowyError> {
        let cloned_ws = self.clone();
        FutureResult::new(async move {
            let _ = cloned_ws.retry(count).await.map_err(internal_error)?;
            Ok(())
        })
    }

    fn add_receiver(&self, receiver: Arc<dyn WSMessageReceiver>) -> Result<(), FlowyError> {
        let _ = self.add_ws_message_receiver(receiver).map_err(internal_error)?;
        Ok(())
    }

    fn get_sender(&self) -> FutureResult<Option<Arc<dyn WebSocketMessageSender>>, FlowyError> {
        let cloned_self = self.clone();
        FutureResult::new(async move {
            match cloned_self.ws_message_sender().await.map_err(internal_error)? {
                None => Ok(None),
                Some(sender) => {
                    let sender = sender as Arc<dyn WebSocketMessageSender>;
                    Ok(Some(sender))
                }
            }
        })
    }
}

impl WebSocketMessageSender for WSSender {
    fn send(&self, msg: WebSocketRawMessage) -> Result<(), FlowyError> {
        let _ = self.send_msg(msg).map_err(internal_error)?;
        Ok(())
    }
}
