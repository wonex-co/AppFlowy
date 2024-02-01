use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
pub use collab_folder::View;
use collab_folder::ViewLayout;
use tokio::sync::RwLock;

use flowy_error::FlowyError;

use flowy_folder_pub::folder_builder::WorkspaceViewBuilder;
use lib_infra::util::timestamp;

use crate::entities::{CreateViewParams, ViewLayoutPB};
use crate::share::ImportType;

pub type ViewData = Bytes;

/// The handler will be used to handler the folder operation for a specific
/// view layout. Each [ViewLayout] will have a handler. So when creating a new
/// view, the [ViewLayout] will be used to get the handler.
///
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait FolderOperationHandler {
  /// Create the view for the workspace of new user.
  /// Only called once when the user is created.
  async fn create_workspace_view(
    &self,
    _uid: i64,
    _workspace_view_builder: Arc<RwLock<WorkspaceViewBuilder>>,
  ) -> Result<(), FlowyError> {
    Ok(())
  }

  /// Closes the view and releases the resources that this view has in
  /// the backend
  async fn close_view(&self, view_id: &str) -> Result<(), FlowyError>;

  /// Called when the view is deleted.
  /// This will called after the view is deleted from the trash.
  async fn delete_view(&self, view_id: &str) -> Result<(), FlowyError>;

  /// Returns the [ViewData] that can be used to create the same view.
  async fn duplicate_view(&self, view_id: &str) -> Result<ViewData, FlowyError>;

  /// Create a view with the data.
  ///
  /// # Arguments
  ///
  /// * `user_id`: the user id
  /// * `view_id`: the view id
  /// * `name`: the name of the view
  /// * `data`: initial data of the view. The data should be parsed by the [FolderOperationHandler]
  /// implementation. For example, the data of the database will be [DatabaseData].
  /// * `layout`: the layout of the view
  /// * `meta`: use to carry extra information. For example, the database view will use this
  /// to carry the reference database id.
  async fn create_view_with_view_data(
    &self,
    user_id: i64,
    view_id: &str,
    name: &str,
    data: Vec<u8>,
    layout: ViewLayout,
    meta: HashMap<String, String>,
  ) -> Result<(), FlowyError>;

  /// Create a view with the pre-defined data.
  /// For example, the initial data of the grid/calendar/kanban board when
  /// you create a new view.
  async fn create_built_in_view(
    &self,
    user_id: i64,
    view_id: &str,
    name: &str,
    layout: ViewLayout,
  ) -> Result<(), FlowyError>;

  /// Create a view by importing data
  async fn import_from_bytes(
    &self,
    uid: i64,
    view_id: &str,
    name: &str,
    import_type: ImportType,
    bytes: Vec<u8>,
  ) -> Result<(), FlowyError>;

  /// Create a view by importing data from a file
  async fn import_from_file_path(
    &self,
    view_id: &str,
    name: &str,
    path: String,
  ) -> Result<(), FlowyError>;

  /// Called when the view is updated. The handler is the `old` registered handler.
  async fn did_update_view(&self, _old: &View, _new: &View) -> Result<(), FlowyError>;
}

pub type FolderOperationHandlers =
  Arc<HashMap<ViewLayout, Arc<dyn FolderOperationHandler + Send + Sync>>>;

impl From<ViewLayoutPB> for ViewLayout {
  fn from(pb: ViewLayoutPB) -> Self {
    match pb {
      ViewLayoutPB::Document => ViewLayout::Document,
      ViewLayoutPB::Grid => ViewLayout::Grid,
      ViewLayoutPB::Board => ViewLayout::Board,
      ViewLayoutPB::Calendar => ViewLayout::Calendar,
    }
  }
}

pub(crate) fn create_view(uid: i64, params: CreateViewParams, layout: ViewLayout) -> View {
  let time = timestamp();
  View {
    id: params.view_id,
    parent_view_id: params.parent_view_id,
    name: params.name,
    desc: params.desc,
    children: Default::default(),
    created_at: time,
    is_favorite: false,
    layout,
    icon: None,
    created_by: Some(uid),
    last_edited_time: 0,
    last_edited_by: Some(uid),
  }
}
