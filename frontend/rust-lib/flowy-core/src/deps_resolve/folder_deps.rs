use bytes::Bytes;
use collab_integrate::collab_builder::AppFlowyCollabBuilder;
use collab_integrate::CollabKVDB;
use flowy_database2::entities::DatabaseLayoutPB;
use flowy_database2::services::share::csv::CSVFormat;
use flowy_database2::template::{make_default_board, make_default_calendar, make_default_grid};
use flowy_database2::DatabaseManager;
use flowy_document::entities::DocumentDataPB;
use flowy_document::manager::DocumentManager;
use flowy_document::parser::json::parser::JsonToDocumentParser;
use flowy_error::FlowyError;
use flowy_folder::entities::ViewLayoutPB;
use flowy_folder::manager::{FolderManager, FolderUser};
use flowy_folder::share::ImportType;
use flowy_folder::view_operation::{FolderOperationHandler, FolderOperationHandlers, View};
use flowy_folder::ViewLayout;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::{Arc, Weak};
use tokio::sync::RwLock;

use flowy_folder_pub::folder_builder::WorkspaceViewBuilder;
use flowy_user::services::authenticate_user::AuthenticateUser;

use crate::integrate::server::ServerProvider;
use lib_dispatch::prelude::ToBytes;
use lib_infra::async_trait;
use lib_infra::async_trait::async_trait;
use lib_infra::future::FutureResult;

pub struct FolderDepsResolver();
impl FolderDepsResolver {
  pub async fn resolve(
    authenticate_user: Weak<AuthenticateUser>,
    document_manager: &Arc<DocumentManager>,
    database_manager: &Arc<DatabaseManager>,
    collab_builder: Arc<AppFlowyCollabBuilder>,
    server_provider: Arc<ServerProvider>,
  ) -> Arc<FolderManager> {
    let user: Arc<dyn FolderUser> = Arc::new(FolderUserImpl {
      authenticate_user: authenticate_user.clone(),
    });

    let handlers = folder_operation_handlers(document_manager.clone(), database_manager.clone());
    Arc::new(
      FolderManager::new(
        user.clone(),
        collab_builder,
        handlers,
        server_provider.clone(),
      )
      .await
      .unwrap(),
    )
  }
}

fn folder_operation_handlers(
  document_manager: Arc<DocumentManager>,
  database_manager: Arc<DatabaseManager>,
) -> FolderOperationHandlers {
  let mut map: HashMap<ViewLayout, Arc<dyn FolderOperationHandler + Send + Sync>> = HashMap::new();

  let document_folder_operation = Arc::new(DocumentFolderOperation(document_manager));
  map.insert(ViewLayout::Document, document_folder_operation);

  let database_folder_operation = Arc::new(DatabaseFolderOperation(database_manager));
  map.insert(ViewLayout::Board, database_folder_operation.clone());
  map.insert(ViewLayout::Grid, database_folder_operation.clone());
  map.insert(ViewLayout::Calendar, database_folder_operation);
  Arc::new(map)
}

struct FolderUserImpl {
  authenticate_user: Weak<AuthenticateUser>,
}

#[async_trait]
impl FolderUser for FolderUserImpl {
  fn user_id(&self) -> Result<i64, FlowyError> {
    self
      .authenticate_user
      .upgrade()
      .ok_or(FlowyError::internal().with_context("Unexpected error: UserSession is None"))?
      .user_id()
  }

  fn collab_db(&self, uid: i64) -> Result<Weak<CollabKVDB>, FlowyError> {
    self
      .authenticate_user
      .upgrade()
      .ok_or(FlowyError::internal().with_context("Unexpected error: UserSession is None"))?
      .get_collab_db(uid)
  }
}

struct DocumentFolderOperation(Arc<DocumentManager>);
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl FolderOperationHandler for DocumentFolderOperation {
  async fn create_workspace_view(
    &self,
    uid: i64,
    workspace_view_builder: Arc<RwLock<WorkspaceViewBuilder>>,
  ) -> Result<(), FlowyError> {
    let mut write_guard = workspace_view_builder.write().await;

    // Create a view named "Getting started" with an icon ⭐️ and the built-in README data.
    // Don't modify this code unless you know what you are doing.
    write_guard
      .with_view_builder(|view_builder| async {
        let view = view_builder
          .with_name("Getting started")
          .with_icon("⭐️")
          .build();
        // create a empty document
        let json_str = include_str!("../../assets/read_me.json");
        let document_pb = JsonToDocumentParser::json_str_to_document(json_str).unwrap();
        self
          .0
          .create_document(uid, &view.parent_view.id, Some(document_pb.into()))
          .await
          .unwrap();
        view
      })
      .await;
    Ok(())
  }

  /// Close the document view.
  async fn close_view(&self, view_id: &str) -> Result<(), FlowyError> {
    self.0.close_document(&view_id).await?;
    Ok(())
  }

  async fn delete_view(&self, view_id: &str) -> Result<(), FlowyError> {
    match self.0.delete_document(&view_id).await {
      Ok(_) => tracing::trace!("Delete document: {}", view_id),
      Err(e) => tracing::error!("🔴delete document failed: {}", e),
    }
    Ok(())
  }

  async fn duplicate_view(&self, view_id: &str) -> Result<Bytes, FlowyError> {
    let data: DocumentDataPB = self.0.get_document_data(&view_id).await?.into();
    let data_bytes = data.into_bytes().map_err(|_| FlowyError::invalid_data())?;
    Ok(data_bytes)
  }

  async fn create_view_with_view_data(
    &self,
    user_id: i64,
    view_id: &str,
    _name: &str,
    data: Vec<u8>,
    layout: ViewLayout,
    _meta: HashMap<String, String>,
  ) -> Result<(), FlowyError> {
    debug_assert_eq!(layout, ViewLayout::Document);
    let data = DocumentDataPB::try_from(Bytes::from(data))?;
    self
      .0
      .create_document(user_id, &view_id, Some(data.into()))
      .await?;
    Ok(())
  }

  /// Create a view with built-in data.
  async fn create_built_in_view(
    &self,
    user_id: i64,
    view_id: &str,
    _name: &str,
    layout: ViewLayout,
  ) -> Result<(), FlowyError> {
    debug_assert_eq!(layout, ViewLayout::Document);
    match self.0.create_document(user_id, &view_id, None).await {
      Ok(_) => Ok(()),
      Err(err) => {
        if err.is_already_exists() {
          Ok(())
        } else {
          Err(err)
        }
      },
    }
  }

  async fn import_from_bytes(
    &self,
    uid: i64,
    view_id: &str,
    _name: &str,
    _import_type: ImportType,
    bytes: Vec<u8>,
  ) -> Result<(), FlowyError> {
    let data = DocumentDataPB::try_from(Bytes::from(bytes))?;
    self
      .0
      .create_document(uid, &view_id, Some(data.into()))
      .await?;
    Ok(())
  }

  // will implement soon
  async fn import_from_file_path(
    &self,
    _view_id: &str,
    _name: &str,
    _path: String,
  ) -> Result<(), FlowyError> {
    Ok(())
  }

  async fn did_update_view(&self, _old: &View, _new: &View) -> Result<(), FlowyError> {
    Ok(())
  }
}

struct DatabaseFolderOperation(Arc<DatabaseManager>);
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl FolderOperationHandler for DatabaseFolderOperation {
  async fn close_view(&self, view_id: &str) -> Result<(), FlowyError> {
    self.0.close_database_view(view_id).await?;
    Ok(())
  }

  async fn delete_view(&self, view_id: &str) -> Result<(), FlowyError> {
    match self.0.delete_database_view(view_id).await {
      Ok(_) => tracing::trace!("Delete database view: {}", view_id),
      Err(e) => tracing::error!("🔴delete database failed: {}", e),
    }
    Ok(())
  }

  async fn duplicate_view(&self, view_id: &str) -> Result<Bytes, FlowyError> {
    let delta_bytes = self.0.duplicate_database(&view_id).await?;
    Ok(Bytes::from(delta_bytes))
  }

  /// Create a database view with duplicated data.
  /// If the ext contains the {"database_id": "xx"}, then it will link
  /// to the existing database.
  async fn create_view_with_view_data(
    &self,
    _user_id: i64,
    view_id: &str,
    name: &str,
    data: Vec<u8>,
    layout: ViewLayout,
    meta: HashMap<String, String>,
  ) -> Result<(), FlowyError> {
    match CreateDatabaseExtParams::from_map(meta) {
      None => {
        self
          .0
          .create_database_with_database_data(&view_id, data)
          .await?;
        Ok(())
      },
      Some(params) => {
        let layout = layout_type_from_view_layout(layout.into());
        self
          .0
          .create_linked_view(
            name.to_string(),
            layout.into(),
            params.database_id,
            view_id.to_string(),
          )
          .await?;
        Ok(())
      },
    }
  }

  /// Create a database view with build-in data.
  /// If the ext contains the {"database_id": "xx"}, then it will link to
  /// the existing database. The data of the database will be shared within
  /// these references views.
  async fn create_built_in_view(
    &self,
    _user_id: i64,
    view_id: &str,
    name: &str,
    layout: ViewLayout,
  ) -> Result<(), FlowyError> {
    let data = match layout {
      ViewLayout::Grid => make_default_grid(view_id, name),
      ViewLayout::Board => make_default_board(view_id, name),
      ViewLayout::Calendar => make_default_calendar(view_id, name),
      ViewLayout::Document => {
        return Err(
          FlowyError::internal().with_context(format!("Can't handle {:?} layout type", layout)),
        );
      },
    };
    let result = self.0.create_database_with_params(data).await;
    match result {
      Ok(_) => Ok(()),
      Err(err) => {
        if err.is_already_exists() {
          Ok(())
        } else {
          Err(err)
        }
      },
    }
  }

  async fn import_from_bytes(
    &self,
    _uid: i64,
    view_id: &str,
    _name: &str,
    import_type: ImportType,
    bytes: Vec<u8>,
  ) -> Result<(), FlowyError> {
    let format = match import_type {
      ImportType::CSV => CSVFormat::Original,
      ImportType::HistoryDatabase => CSVFormat::META,
      ImportType::RawDatabase => CSVFormat::META,
      _ => CSVFormat::Original,
    };
    let content =
      String::from_utf8(bytes).map_err(|err| FlowyError::internal().with_context(err))?;
    self
      .0
      .import_csv(view_id.to_string(), content, format)
      .await?;
    Ok(())
  }

  async fn import_from_file_path(
    &self,
    _view_id: &str,
    _name: &str,
    path: String,
  ) -> Result<(), FlowyError> {
    self.0.import_csv_from_file(path, CSVFormat::META).await?;
    Ok(())
  }

  async fn did_update_view(&self, old: &View, new: &View) -> Result<(), FlowyError> {
    let database_layout = match new.layout {
      ViewLayout::Document => {
        return Err(FlowyError::internal().with_context("Can't handle document layout type"));
      },
      ViewLayout::Grid => DatabaseLayoutPB::Grid,
      ViewLayout::Board => DatabaseLayoutPB::Board,
      ViewLayout::Calendar => DatabaseLayoutPB::Calendar,
    };

    if old.layout != new.layout {
      self
        .0
        .update_database_layout(&new.id, database_layout)
        .await?;
      Ok(())
    } else {
      Ok(())
    }
  }
}

#[derive(Debug, serde::Deserialize)]
struct CreateDatabaseExtParams {
  database_id: String,
}

impl CreateDatabaseExtParams {
  pub fn from_map(map: HashMap<String, String>) -> Option<Self> {
    let value = serde_json::to_value(map).ok()?;
    serde_json::from_value::<Self>(value).ok()
  }
}

pub fn layout_type_from_view_layout(layout: ViewLayoutPB) -> DatabaseLayoutPB {
  match layout {
    ViewLayoutPB::Grid => DatabaseLayoutPB::Grid,
    ViewLayoutPB::Board => DatabaseLayoutPB::Board,
    ViewLayoutPB::Calendar => DatabaseLayoutPB::Calendar,
    ViewLayoutPB::Document => DatabaseLayoutPB::Grid,
  }
}
