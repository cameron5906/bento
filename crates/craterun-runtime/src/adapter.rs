use std::path::Path;

use async_trait::async_trait;

use craterun_core::{AppId, CrateRunError};

use crate::types::{ContainerStatus, ImageRef, LogLine, RemoveOptions, RuntimeDetectionResult, RuntimePlan};

#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    async fn detect(&self) -> Result<RuntimeDetectionResult, CrateRunError>;

    async fn prepare(&self) -> Result<(), CrateRunError>;

    async fn import_image(
        &self,
        archive_path: &Path,
        image_name: &str,
    ) -> Result<ImageRef, CrateRunError>;

    async fn start_app(&self, plan: &RuntimePlan) -> Result<(), CrateRunError>;

    async fn stop_app(&self, app_id: &AppId) -> Result<(), CrateRunError>;

    async fn get_container_status(
        &self,
        app_id: &AppId,
        service: &str,
    ) -> Result<ContainerStatus, CrateRunError>;

    async fn get_logs(
        &self,
        app_id: &AppId,
        service: Option<&str>,
        tail: Option<usize>,
    ) -> Result<Vec<LogLine>, CrateRunError>;

    async fn create_network(&self, app_id: &AppId) -> Result<(), CrateRunError>;

    async fn create_volume(
        &self,
        app_id: &AppId,
        volume_name: &str,
    ) -> Result<(), CrateRunError>;

    async fn remove_app(
        &self,
        app_id: &AppId,
        options: RemoveOptions,
    ) -> Result<(), CrateRunError>;

    fn adapter_name(&self) -> &'static str;
}
