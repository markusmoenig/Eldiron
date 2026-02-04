use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Project data model that can be serialized to/from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project metadata
    pub metadata: ProjectMetadata,

    /// Application-specific data (can be customized per app)
    pub data: serde_json::Value,

    /// Project file path (not serialized, set on load/save)
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project name
    pub name: String,

    /// Project version (semantic versioning)
    pub version: String,

    /// App version that created this project
    pub app_version: String,

    /// Creation timestamp (Unix timestamp)
    pub created_at: u64,

    /// Last modified timestamp (Unix timestamp)
    pub modified_at: u64,

    /// Optional author name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Project {
    /// Create a new empty project
    pub fn new(name: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: ProjectMetadata {
                name: name.into(),
                version: "1.0.0".to_string(),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                created_at: now,
                modified_at: now,
                author: None,
                description: None,
            },
            data: serde_json::Value::Null,
            file_path: None,
        }
    }

    /// Create a new project with custom data
    pub fn with_data(name: impl Into<String>, data: serde_json::Value) -> Self {
        let mut project = Self::new(name);
        project.data = data;
        project
    }

    /// Load project from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Load project from JSON file
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self, ProjectError> {
        let path = path.into();
        let json = std::fs::read_to_string(&path)?;
        let mut project: Project = serde_json::from_str(&json)?;
        project.file_path = Some(path);
        Ok(project)
    }

    /// Serialize project to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Save project to JSON file
    pub fn to_file(&mut self, path: impl Into<PathBuf>) -> Result<(), ProjectError> {
        let path = path.into();
        let json = self.to_json()?;
        std::fs::write(&path, json)?;

        // Update metadata
        self.file_path = Some(path);
        self.metadata.modified_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(())
    }

    /// Save to current file path (if set)
    pub fn save(&mut self) -> Result<(), ProjectError> {
        if let Some(path) = self.file_path.clone() {
            self.to_file(path)
        } else {
            Err(ProjectError::NoFilePath)
        }
    }

    /// Update modification timestamp
    pub fn mark_modified(&mut self) {
        self.metadata.modified_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

#[derive(Debug)]
pub enum ProjectError {
    Io(std::io::Error),
    Json(serde_json::Error),
    NoFilePath,
}

impl From<std::io::Error> for ProjectError {
    fn from(err: std::io::Error) -> Self {
        ProjectError::Io(err)
    }
}

impl From<serde_json::Error> for ProjectError {
    fn from(err: serde_json::Error) -> Self {
        ProjectError::Json(err)
    }
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Io(e) => write!(f, "IO error: {}", e),
            ProjectError::Json(e) => write!(f, "JSON error: {}", e),
            ProjectError::NoFilePath => write!(f, "No file path set for project"),
        }
    }
}

impl std::error::Error for ProjectError {}
