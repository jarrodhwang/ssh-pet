use crate::{
    error::AppError,
    model::{Connection, Preferences},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionDraft {
    pub id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    /// Kept as input text until Rust parses and validates it.
    pub port: String,
    pub identity_file: String,
}
impl Default for ConnectionDraft {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            host: String::new(),
            username: String::new(),
            port: "22".into(),
            identity_file: String::new(),
        }
    }
}
impl From<&Connection> for ConnectionDraft {
    fn from(c: &Connection) -> Self {
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            host: c.host.clone(),
            username: c.username.clone(),
            port: c.port.to_string(),
            identity_file: c.identity_file.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MainRequest {
    View { query: String },
    NewDraft,
    Save { draft: ConnectionDraft },
    Import { name: String, command: String },
    Delete { id: String },
    Favorite { id: String },
    Preferences { preferences: Preferences },
    LaunchLock { locked: bool },
    Connect { id: String },
    Diagnose { id: String },
    CancelDiagnostics,
    ResetPet,
    StartAtLogin { enabled: bool },
    HideLauncher,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PetRequest {
    View,
    OpenLauncher,
    ConnectFavorite,
    BeginDrag { x: f64, y: f64 },
    Drag { x: f64, y: f64 },
    EndDrag,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PetMood {
    Idle,
    Checking,
    Opening,
    Attention,
    Locked,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PetView {
    pub pet: crate::pets::PetKind,
    pub name: String,
    pub theme: crate::pets::PetTheme,
    pub mood: PetMood,
    pub tooltip: String,
    pub status: String,
    pub reduce_motion: bool,
    pub can_connect: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    Pass,
    Warning,
    Fail,
    Info,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct Check {
    pub label: String,
    pub status: CheckStatus,
    pub message: String,
}
impl Check {
    pub fn new(label: &str, status: CheckStatus, message: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub connection_id: String,
    pub summary: String,
    pub checks: Vec<Check>,
    pub elapsed_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Activity {
    pub id: String,
    #[ts(type = "number")]
    pub timestamp: u64,
    pub action: String,
    pub outcome: String,
    pub connection_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub id: String,
    pub name: String,
    pub address: String,
    pub auth_label: String,
    pub port_label: String,
    pub is_favorite: bool,
    pub can_connect: bool,
    pub connect_label: String,
    pub can_diagnose: bool,
    pub draft: ConnectionDraft,
    pub report: Option<DiagnosticReport>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppView {
    pub pet_themes: Vec<crate::pets::PetThemeView>,
    pub connections: Vec<ConnectionView>,
    pub connection_count: usize,
    pub preferences: Preferences,
    pub launch_locked: bool,
    pub diagnostic_running: bool,
    pub activity: Vec<Activity>,
    pub notice: Option<String>,
    pub load_error: Option<AppError>,
    pub pet: PetView,
    pub security_summary: Vec<String>,
    pub start_at_login: bool,
    pub platform: String,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Reply {
    View(AppView),
    Draft(ConnectionDraft),
    Done,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum PetReply {
    View(PetView),
    Done,
}
