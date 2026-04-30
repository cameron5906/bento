use std::path::Path;

use async_trait::async_trait;

use bento_core::{AppId, BentoError};

use crate::types::{ContainerStatus, ImageRef, LogLine, RemoveOptions, RuntimeDetectionResult, RuntimePlan};

#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    async fn detect(&self) -> Result<RuntimeDetectionResult, BentoError>;

    async fn prepare(&self) -> Result<(), BentoError>;

    async fn import_image(
        &self,
        archive_path: &Path,
        image_name: &str,
    ) -> Result<ImageRef, BentoError>;

    async fn start_app(&self, plan: &RuntimePlan) -> Result<(), BentoError>;

    async fn stop_app(&self, app_id: &AppId) -> Result<(), BentoError>;

    async fn get_container_status(
        &self,
        app_id: &AppId,
        service: &str,
    ) -> Result<ContainerStatus, BentoError>;

    async fn get_logs(
        &self,
        app_id: &AppId,
        service: Option<&str>,
        tail: Option<usize>,
    ) -> Result<Vec<LogLine>, BentoError>;

    async fn create_network(&self, app_id: &AppId) -> Result<(), BentoError>;

    async fn create_volume(
        &self,
        app_id: &AppId,
        volume_name: &str,
    ) -> Result<(), BentoError>;

    async fn remove_app(
        &self,
        app_id: &AppId,
        options: RemoveOptions,
    ) -> Result<(), BentoError>;

    fn adapter_name(&self) -> &'static str;
}
