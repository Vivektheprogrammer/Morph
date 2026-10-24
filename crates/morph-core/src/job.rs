use crate::format::Format;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Validating,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub source_paths: Vec<PathBuf>,
    pub target_format: Format,
    pub status: JobStatus,
    pub progress_percent: u8,
    pub output_path: Option<PathBuf>,
    pub message: String,
}

impl Job {
    pub fn new(source_paths: Vec<PathBuf>, target_format: Format) -> Self {
        let id = format!(
            "job_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        Self {
            id,
            source_paths,
            target_format,
            status: JobStatus::Queued,
            progress_percent: 0,
            output_path: None,
            message: "Queued for processing".to_string(),
        }
    }

    pub fn set_running(&mut self, progress: u8, msg: &str) {
        self.status = JobStatus::Running;
        self.progress_percent = progress.min(100);
        self.message = msg.to_string();
    }

    pub fn set_validating(&mut self) {
        self.status = JobStatus::Validating;
        self.progress_percent = 95;
        self.message = "Validating generated file".to_string();
    }

    pub fn set_completed(&mut self, output: PathBuf) {
        self.status = JobStatus::Completed;
        self.progress_percent = 100;
        self.output_path = Some(output);
        self.message = "Conversion succeeded".to_string();
    }

    pub fn set_failed(&mut self, error: &str) {
        self.status = JobStatus::Failed(error.to_string());
        self.message = error.to_string();
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Completed | JobStatus::Failed(_) | JobStatus::Cancelled
        )
    }
}
