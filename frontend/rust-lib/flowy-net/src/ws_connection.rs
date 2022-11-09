use crate::entities::NetworkType;

pub use flowy_error::FlowyError;
use lib_infra::future::FutureResult;
pub use lib_ws::{WSConnectState, WSMessageReceiver, WebSocketRawMessage};

use futures_util::future::BoxFuture;
use lib_ws::WSController;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;

pub trait RawWebSocket: Send + Sync {
    fn initialize(&self) -> FutureResult<(), FlowyError>;
    fn start_connect(&self, addr: String, user_id: String) -> FutureResult<(), FlowyError>;
    fn stop_connect(&self) -> FutureResult<(), FlowyError>;
    fn subscribe_connect_state(&self) -> BoxFuture<broadcast::Receiver<WSConnectState>>;
    fn reconnect(&self, count: usize) -> FutureResult<(), FlowyError>;
    fn add_receiver(&self, receiver: Arc<dyn WSMessageReceiver>) -> Result<(), FlowyError>;
    fn get_sender(&self) -> FutureResult<Option<Arc<dyn WebSocketMessageSender>>, FlowyError>;
}

pub trait WebSocketMessageSender: Send + Sync {
    fn send(&self, msg: WebSocketRawMessage) -> Result<(), FlowyError>;
}

pub struct WebSocketConnect {
    inner: Arc<dyn RawWebSocket>,
    connect_type: RwLock<NetworkType>,
    status_notifier: broadcast::Sender<NetworkType>,
    addr: String,
}

impl WebSocketConnect {
    pub fn new(addr: String) -> Self {
        let ws = Arc::new(Arc::new(WSController::new()));
        let (status_notifier, _) = broadcast::channel(10);
        WebSocketConnect {
            inner: ws,
            connect_type: RwLock::new(NetworkType::default()),
            status_notifier,
            addr,
        }
    }

    pub fn from_local(addr: String, ws: Arc<dyn RawWebSocket>) -> Self {
        let (status_notifier, _) = broadcast::channel(10);
        WebSocketConnect {
            inner: ws,
            connect_type: RwLock::new(NetworkType::default()),
            status_notifier,
            addr,
        }
    }

    pub async fn init(&self) {
        match self.inner.initialize().await {
            Ok(_) => {}
            Err(e) => tracing::error!("FlowyWebSocketConnect init error: {:?}", e),
        }
    }

    pub async fn start(&self, token: String, user_id: String) -> Result<(), FlowyError> {
        let addr = format!("{}/{}", self.addr, &token);
        self.inner.stop_connect().await?;
        let _ = self.inner.start_connect(addr, user_id).await?;
        Ok(())
    }

    pub async fn stop(&self) {
        let _ = self.inner.stop_connect().await;
    }

    pub fn update_network_type(&self, new_type: &NetworkType) {
        tracing::debug!("Network new state: {:?}", new_type);
        let old_type = self.connect_type.read().clone();
        let _ = self.status_notifier.send(new_type.clone());

        if &old_type != new_type {
            tracing::debug!("Connect type switch from {:?} to {:?}", old_type, new_type);
            match (old_type.is_connect(), new_type.is_connect()) {
                (false, true) => {
                    let ws_controller = self.inner.clone();
                    tokio::spawn(async move { retry_connect(ws_controller, 100).await });
                }
                (true, false) => {
                    //
                }
                _ => {}
            }

            *self.connect_type.write() = new_type.clone();
        }
    }

    pub async fn subscribe_websocket_state(&self) -> broadcast::Receiver<WSConnectState> {
        self.inner.subscribe_connect_state().await
    }

    pub fn subscribe_network_ty(&self) -> broadcast::Receiver<NetworkType> {
        self.status_notifier.subscribe()
    }

    pub fn add_ws_message_receiver(&self, receiver: Arc<dyn WSMessageReceiver>) -> Result<(), FlowyError> {
        let _ = self.inner.add_receiver(receiver)?;
        Ok(())
    }

    pub async fn web_socket(&self) -> Result<Option<Arc<dyn WebSocketMessageSender>>, FlowyError> {
        self.inner.get_sender().await
    }
}

#[tracing::instrument(level = "debug", skip(ws_conn))]
pub fn listen_on_websocket(ws_conn: Arc<WebSocketConnect>) {
    let raw_web_socket = ws_conn.inner.clone();
    let _ = tokio::spawn(async move {
        let mut notify = ws_conn.inner.subscribe_connect_state().await;
        loop {
            match notify.recv().await {
                Ok(state) => {
                    tracing::info!("Websocket state changed: {}", state);
                    match state {
                        WSConnectState::Init => {}
                        WSConnectState::Connected => {}
                        WSConnectState::Connecting => {}
                        WSConnectState::Disconnected => retry_connect(raw_web_socket.clone(), 100).await,
                    }
                }
                Err(e) => {
                    tracing::error!("Websocket state notify error: {:?}", e);
                    break;
                }
            }
        }
    });
}

async fn retry_connect(ws: Arc<dyn RawWebSocket>, count: usize) {
    match ws.reconnect(count).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("websocket connect failed: {:?}", e);
        }
    }
}
