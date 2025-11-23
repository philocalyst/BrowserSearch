// Define error types compatible with alfrusco
#[derive(Debug, thiserror::Error)]
pub enum WorkflowErrorType {
    #[error("Search error: {0}")]
    Search(#[from] Box<dyn Error>),
    #[error("Tab management error: {0}")]
    Tab(#[from] tabs::TabError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl WorkflowError for WorkflowErrorType {}
