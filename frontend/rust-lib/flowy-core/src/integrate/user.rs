use std::sync::Arc;

use anyhow::Context;
use collab_entity::CollabType;
use tracing::event;

use collab_integrate::collab_builder::AppFlowyCollabBuilder;
use flowy_database2::DatabaseManager;
use flowy_document::manager::DocumentManager;
use flowy_error::FlowyResult;
use flowy_folder::manager::{FolderInitDataSource, FolderManager};
use flowy_user::event_map::UserStatusCallback;
use flowy_user_pub::cloud::{UserCloudConfig, UserCloudServiceProvider};
use flowy_user_pub::entities::{Authenticator, UserProfile, UserWorkspace};
use lib_infra::async_trait;
use lib_infra::future::{to_fut, Fut};

use crate::integrate::server::{Server, ServerProvider};
use crate::AppFlowyCoreConfig;

pub(crate) struct UserStatusCallbackImpl {
  pub(crate) collab_builder: Arc<AppFlowyCollabBuilder>,
  pub(crate) folder_manager: Arc<FolderManager>,
  pub(crate) database_manager: Arc<DatabaseManager>,
  pub(crate) document_manager: Arc<DocumentManager>,
  pub(crate) server_provider: Arc<ServerProvider>,
  #[allow(dead_code)]
  pub(crate) config: AppFlowyCoreConfig,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl UserStatusCallback for UserStatusCallbackImpl {
  async fn did_init(
    &self,
    user_id: i64,
    user_authenticator: &Authenticator,
    cloud_config: &Option<UserCloudConfig>,
    user_workspace: &UserWorkspace,
    _device_id: &str,
  ) -> FlowyResult<()> {
    self
      .server_provider
      .set_user_authenticator(user_authenticator);

    if let Some(cloud_config) = cloud_config {
      self
        .server_provider
        .set_enable_sync(user_id, cloud_config.enable_sync);
      if cloud_config.enable_encrypt {
        self
          .server_provider
          .set_encrypt_secret(cloud_config.encrypt_secret.clone());
      }
    }

    self.collab_builder.initialize(user_workspace.id.clone());
    self
      .folder_manager
      .initialize(
        user_id,
        &user_workspace.id,
        FolderInitDataSource::LocalDisk {
          create_if_not_exist: false,
        },
      )
      .await?;
    self
      .database_manager
      .initialize(
        user_id,
        user_workspace.id.clone(),
        user_workspace.database_view_tracker_id.clone(),
      )
      .await?;
    self
      .document_manager
      .initialize(user_id, user_workspace.id.clone())
      .await?;
    Ok(())
  }

  async fn did_sign_in(
    &self,
    user_id: i64,
    user_workspace: &UserWorkspace,
    device_id: &str,
  ) -> FlowyResult<()> {
    event!(
      tracing::Level::TRACE,
      "Notify did sign in: latest_workspace: {:?}, device_id: {}",
      user_workspace,
      device_id
    );

    self
      .folder_manager
      .initialize_with_workspace_id(user_id, &user_workspace.id)
      .await?;
    self
      .database_manager
      .initialize(
        user_id,
        user_workspace.id.clone(),
        user_workspace.database_view_tracker_id.clone(),
      )
      .await?;
    self
      .document_manager
      .initialize(user_id, user_workspace.id.clone())
      .await?;
    Ok(())
  }

  async fn did_sign_up(
    &self,
    is_new_user: bool,
    user_profile: &UserProfile,
    user_workspace: &UserWorkspace,
    device_id: &str,
  ) -> FlowyResult<()> {
    self
      .server_provider
      .set_user_authenticator(&user_profile.authenticator);
    let server_type = self.server_provider.get_server_type();

    event!(
      tracing::Level::TRACE,
      "Notify did sign up: is new: {} user_workspace: {:?}, device_id: {}",
      is_new_user,
      user_workspace,
      device_id
    );

    // In the current implementation, when a user signs up for AppFlowy Cloud, a default workspace
    // is automatically created for them. However, for users who sign up through Supabase, the creation
    // of the default workspace relies on the client-side operation. This means that the process
    // for initializing a default workspace differs depending on the sign-up method used.
    let data_source = match self
      .folder_manager
      .cloud_service
      .get_collab_doc_state_f(
        &user_workspace.id,
        user_profile.uid,
        CollabType::Folder,
        &user_workspace.id,
      )
      .await
    {
      Ok(doc_state) => match server_type {
        Server::Local => FolderInitDataSource::LocalDisk {
          create_if_not_exist: true,
        },
        Server::AppFlowyCloud => FolderInitDataSource::Cloud(doc_state),
        Server::Supabase => {
          if is_new_user {
            FolderInitDataSource::LocalDisk {
              create_if_not_exist: true,
            }
          } else {
            FolderInitDataSource::Cloud(doc_state)
          }
        },
      },
      Err(_) => FolderInitDataSource::LocalDisk {
        create_if_not_exist: true,
      },
    };

    self
      .folder_manager
      .initialize_with_new_user(
        user_profile.uid,
        &user_profile.token,
        is_new_user,
        data_source,
        &user_workspace.id,
      )
      .await
      .context("FolderManager error")?;

    self
      .database_manager
      .initialize_with_new_user(
        user_profile.uid,
        user_workspace.id.clone(),
        user_workspace.database_view_tracker_id.clone(),
      )
      .await
      .context("DatabaseManager error")?;

    self
      .document_manager
      .initialize_with_new_user(user_profile.uid, user_workspace.id.clone())
      .await
      .context("DocumentManager error")?;
    Ok(())
  }

  async fn did_expired(&self, _token: &str, user_id: i64) -> FlowyResult<()> {
    self.folder_manager.clear(user_id).await;
    Ok(())
  }

  async fn open_workspace(&self, user_id: i64, user_workspace: &UserWorkspace) -> FlowyResult<()> {
    self.collab_builder.initialize(user_workspace.id.clone());
    self
      .folder_manager
      .initialize_with_workspace_id(user_id, &user_workspace.id)
      .await?;

    self
      .database_manager
      .initialize(
        user_id,
        user_workspace.id.clone(),
        user_workspace.database_view_tracker_id.clone(),
      )
      .await?;
    self
      .document_manager
      .initialize(user_id, user_workspace.id.clone())
      .await?;
    Ok(())
  }

  fn did_update_network(&self, reachable: bool) {
    self.collab_builder.update_network(reachable);
  }
}
