use crate::{
    diagnostics,
    model::{self, Config, Connection, PetPosition, Preferences, TerminalShell},
    protocol::*,
    security::{self, LaunchSpec},
    storage, AppError, ErrorCode, Result,
};
use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;

const COOLDOWN: Duration = Duration::from_secs(2);
const ACTIVITY_LIMIT: usize = 200;
type Notifier = Arc<dyn Fn() + Send + Sync>;

struct State {
    config: Config,
    activity: Vec<Activity>,
    reports: HashMap<String, DiagnosticReport>,
    load_error: Option<AppError>,
    notice: Option<String>,
    attention: bool,
    launching: Option<String>,
    last_launch: Option<Instant>,
    diagnostic: Option<(String, watch::Sender<bool>)>,
    last_diagnostic: Option<Instant>,
}

/// The application domain. Neither React nor Tauri owns any connection decisions.
pub struct Core {
    state: Mutex<State>,
    directory: PathBuf,
    home: PathBuf,
    notify: Mutex<Option<Notifier>>,
}

impl Core {
    pub fn open(directory: PathBuf, home: PathBuf) -> Arc<Self> {
        let path = directory.join("connections.json");
        let mut notice = None;
        let mut load_error = storage::private_directory(&directory).err();
        let config = match storage::load(&path) {
            Ok(Some(config)) => config,
            Ok(None) => {
                let mut config = Config::default();
                let launcher = home.join("Desktop/zbook-studio-ssh.command");
                if let Ok(file) = open_launcher(&launcher) {
                    let mut text = String::new();
                    if file.take(8193).read_to_string(&mut text).is_ok() {
                        match model::parse_launcher(&text, "Zbook Studio") {
                            Ok(connection) => {
                                config.favorite_id = Some(connection.id.clone());
                                config.connections.push(connection);
                                notice = Some("Your Zbook Studio connection was imported from the Desktop launcher.".into());
                            }
                            Err(error) => notice = Some(format!("Desktop import skipped: {error}")),
                        }
                    }
                }
                if load_error.is_none() {
                    load_error = storage::save(&path, &config).err();
                }
                config
            }
            Err(error) => {
                load_error = Some(error);
                Config::default()
            }
        };
        let activity = match storage::read_json::<Vec<Activity>>(&directory.join("activity.json")) {
            Ok(Some(entries))
                if entries.len() <= ACTIVITY_LIMIT
                    && entries.iter().all(|e| {
                        e.action.len() < 100
                            && e.outcome.len() < 100
                            && e.timestamp <= 8_640_000_000_000_000
                            && uuid::Uuid::parse_str(&e.id).is_ok()
                    }) =>
            {
                entries
            }
            Ok(None) => vec![],
            Ok(Some(_)) => {
                load_error = Some(AppError::storage(
                    "Activity history is invalid; the file was preserved.",
                ));
                vec![]
            }
            Err(error) => {
                load_error = Some(error);
                vec![]
            }
        };
        Arc::new(Self {
            directory,
            home,
            notify: Mutex::new(None),
            state: Mutex::new(State {
                config,
                activity,
                reports: HashMap::new(),
                load_error,
                notice,
                attention: false,
                launching: None,
                last_launch: None,
                diagnostic: None,
                last_diagnostic: None,
            }),
        })
    }

    fn state(&self) -> Result<MutexGuard<'_, State>> {
        self.state
            .lock()
            .map_err(|_| AppError::storage("Application state is unavailable."))
    }
    pub fn set_notifier(&self, notify: impl Fn() + Send + Sync + 'static) {
        if let Ok(mut slot) = self.notify.lock() {
            *slot = Some(Arc::new(notify));
        }
    }
    fn changed(&self) {
        let notify = self.notify.lock().ok().and_then(|n| n.clone());
        if let Some(notify) = notify {
            notify();
        }
    }
    pub fn config(&self) -> Result<Config> {
        Ok(self.state()?.config.clone())
    }
    fn writable(state: &State) -> Result<()> {
        state.load_error.clone().map_or(Ok(()), Err)
    }
    fn find(state: &State, id: &str) -> Result<Connection> {
        state
            .config
            .connections
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "This connection no longer exists."))
    }

    fn record(
        &self,
        state: &mut State,
        action: &str,
        outcome: &str,
        id: Option<&str>,
    ) -> Result<()> {
        let mut entries = state.activity.clone();
        entries.insert(
            0,
            Activity {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                action: action.into(),
                outcome: outcome.into(),
                connection_id: id.map(str::to_owned),
            },
        );
        entries.truncate(ACTIVITY_LIMIT);
        storage::write_json(&self.directory.join("activity.json"), &entries)?;
        state.activity = entries;
        Ok(())
    }

    pub fn save_position(&self, position: Option<PetPosition>) -> Result<()> {
        let mut state = self.state()?;
        Self::writable(&state)?;
        let mut config = state.config.clone();
        config.pet_position = position;
        storage::save(&self.directory.join("connections.json"), &config)?;
        state.config = config;
        Ok(())
    }

    fn pet_view(state: &State) -> PetView {
        let favorite = state
            .config
            .connections
            .iter()
            .find(|c| Some(&c.id) == state.config.favorite_id.as_ref());
        let (mood, status) = if state.config.launch_locked {
            (PetMood::Locked, "SSH launches paused")
        } else if state.launching.is_some() {
            (PetMood::Opening, "Opening Terminal…")
        } else if state.diagnostic.is_some() {
            (PetMood::Checking, "Checking connection…")
        } else if state.attention || state.load_error.is_some() {
            (PetMood::Attention, "Needs a little attention")
        } else {
            (PetMood::Idle, "Ready when you are")
        };
        let can_connect = favorite.is_some()
            && !state.config.launch_locked
            && state.launching.is_none()
            && state.load_error.is_none();
        PetView {
            pet: state.config.pet,
            name: state.config.pet.name().into(),
            theme: state.config.pet.theme(),
            mood,
            status: status.into(),
            reduce_motion: state.config.reduce_motion,
            can_connect,
        }
    }
    pub fn pet(&self) -> Result<PetView> {
        let state = self.state()?;
        Ok(Self::pet_view(&state))
    }

    pub fn view(&self, query: &str) -> Result<AppView> {
        if query.len() > 256 {
            return Err(AppError::field(
                "query",
                "Search is limited to 256 characters.",
            ));
        }
        let state = self.state()?;
        let query = query.trim().to_lowercase();
        let can_connect =
            !state.config.launch_locked && state.launching.is_none() && state.load_error.is_none();
        let connections = state
            .config
            .connections
            .iter()
            .filter(|c| {
                format!("{} {} {}", c.name, c.host, c.username)
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|c| ConnectionView {
                id: c.id.clone(),
                name: c.name.clone(),
                address: if c.port == 22 {
                    c.destination()
                } else {
                    format!("{}:{}", c.destination(), c.port)
                },
                auth_label: if c.identity_file.is_empty() {
                    "SSH config / agent"
                } else {
                    "SSH key"
                }
                .into(),
                port_label: format!("Port {}", c.port),
                is_favorite: Some(&c.id) == state.config.favorite_id.as_ref(),
                can_connect,
                connect_label: if state.config.launch_locked {
                    "Paused"
                } else if state.launching.as_deref() == Some(&c.id) {
                    "Opening…"
                } else {
                    "Connect"
                }
                .into(),
                can_diagnose: state.diagnostic.is_none() && state.load_error.is_none(),
                draft: c.into(),
                report: state.reports.get(&c.id).cloned(),
            })
            .collect();
        Ok(AppView {
            pet_themes: crate::pets::catalog(),
            connections,
            connection_count: state.config.connections.len(),
            preferences: Preferences {
                pet: state.config.pet,
                pet_visible: state.config.pet_visible,
                reduce_motion: state.config.reduce_motion,
                terminal_shell: state.config.terminal_shell.clone(),
            },
            launch_locked: state.config.launch_locked,
            diagnostic_running: state.diagnostic.is_some(),
            activity: state.activity.clone(),
            notice: state.notice.clone(),
            load_error: state.load_error.clone(),
            pet: Self::pet_view(&state),
            start_at_login: false,
            platform: std::env::consts::OS.into(),
            security_summary: vec![
                "Host-key verification enforced".into(),
                "Agent / X11 forwarding disabled".into(),
                "LocalCommand and implicit tunnels disabled".into(),
                "Explicit key permissions checked".into(),
                "Launch cooldown and window permissions enforced".into(),
            ],
        })
    }

    pub async fn execute(
        self: &Arc<Self>,
        request: MainRequest,
        allow_network: bool,
    ) -> Result<Reply> {
        match request {
            MainRequest::View { query } => return self.view(&query).map(Reply::View),
            MainRequest::NewDraft => return Ok(Reply::Draft(ConnectionDraft::default())),
            MainRequest::Diagnose { id } => {
                self.diagnose(&id, allow_network).await?;
                return Ok(Reply::Done);
            }
            MainRequest::CancelDiagnostics => {
                if let Some((_, cancel)) = &self.state()?.diagnostic {
                    let _ = cancel.send(true);
                }
                return Ok(Reply::Done);
            }
            MainRequest::Connect { .. }
            | MainRequest::StartAtLogin { .. }
            | MainRequest::ResetPet
            | MainRequest::HideLauncher => {
                return Err(AppError::new(
                    ErrorCode::Unsupported,
                    "This action requires the desktop app.",
                ))
            }
            request => {
                let mut state = self.state()?;
                Self::writable(&state)?;
                let mut next = state.config.clone();
                let (action, id) = match request {
                    MainRequest::Save { draft } => {
                        let mut connection = Connection {
                            id: draft.id,
                            name: draft.name.trim().into(),
                            host: draft.host.trim().into(),
                            username: draft.username.trim().into(),
                            port: draft.port.trim().parse().map_err(|_| {
                                AppError::field("port", "Enter a port from 1 to 65535.")
                            })?,
                            identity_file: draft.identity_file.trim().into(),
                        };
                        connection.validate().map_err(AppError::from)?;
                        if connection.id.is_empty() {
                            connection.id = uuid::Uuid::new_v4().to_string();
                            if next.connections.is_empty() {
                                next.favorite_id = Some(connection.id.clone());
                            }
                            next.connections.push(connection.clone());
                        } else {
                            let current = next
                                .connections
                                .iter_mut()
                                .find(|c| c.id == connection.id)
                                .ok_or_else(|| {
                                    AppError::new(
                                        ErrorCode::NotFound,
                                        "This connection no longer exists.",
                                    )
                                })?;
                            *current = connection.clone();
                        }
                        ("connection saved", Some(connection.id))
                    }
                    MainRequest::Import { name, command } => {
                        let c =
                            model::parse_launcher(&command, name.trim()).map_err(AppError::from)?;
                        let id = c.id.clone();
                        if next.connections.is_empty() {
                            next.favorite_id = Some(id.clone());
                        }
                        next.connections.push(c);
                        ("connection imported", Some(id))
                    }
                    MainRequest::Delete { id } => {
                        Self::find(&state, &id)?;
                        next.connections.retain(|c| c.id != id);
                        if next.favorite_id.as_ref() == Some(&id) {
                            next.favorite_id = next.connections.first().map(|c| c.id.clone());
                        }
                        ("connection removed", Some(id))
                    }
                    MainRequest::Favorite { id } => {
                        Self::find(&state, &id)?;
                        next.favorite_id = Some(id.clone());
                        ("favorite changed", Some(id))
                    }
                    MainRequest::Preferences { preferences } => {
                        next.pet = preferences.pet;
                        next.pet_visible = preferences.pet_visible;
                        next.reduce_motion = preferences.reduce_motion;
                        next.terminal_shell = preferences.terminal_shell;
                        ("preferences changed", None)
                    }
                    MainRequest::LaunchLock { locked } => {
                        next.launch_locked = locked;
                        (
                            if locked {
                                "launches paused"
                            } else {
                                "launches resumed"
                            },
                            None,
                        )
                    }
                    _ => unreachable!("non-mutating requests returned above"),
                };
                next.validate().map_err(AppError::from)?;
                // Intent is recorded before the write. History does not claim an uncommitted action succeeded.
                self.record(&mut state, action, "requested", id.as_deref())?;
                storage::save(&self.directory.join("connections.json"), &next)?;
                state.config = next;
                state.attention = false;
                if let Some(id) = &id {
                    state.reports.remove(id);
                }
            }
        }
        self.changed();
        Ok(Reply::Done)
    }

    pub fn begin_launch(self: &Arc<Self>, id: Option<&str>) -> Result<LaunchLease> {
        let result = (|| {
            let mut state = self.state()?;
            Self::writable(&state)?;
            if state.config.launch_locked {
                return Err(AppError::new(
                    ErrorCode::Locked,
                    "SSH launches are paused. Resume them in Security before connecting.",
                ));
            }
            if state.launching.is_some() {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "Terminal is already opening.",
                ));
            }
            if state
                .last_launch
                .is_some_and(|last| last.elapsed() < COOLDOWN)
            {
                return Err(AppError::new(
                    ErrorCode::RateLimited,
                    "Wait two seconds between launch requests.",
                ));
            }
            let id = id.or(state.config.favorite_id.as_deref()).ok_or_else(|| {
                AppError::new(ErrorCode::NotFound, "Add a favorite connection first.")
            })?;
            let c = Self::find(&state, id)?;
            let spec = security::launch_spec(&c, &self.home)?;
            self.record(&mut state, "terminal launch", "requested", Some(&c.id))?;
            state.last_launch = Some(Instant::now());
            state.launching = Some(c.id.clone());
            state.attention = false;
            Ok(LaunchLease {
                core: self.clone(),
                id: c.id,
                spec,
                terminal_shell: state.config.terminal_shell.clone(),
                finished: false,
            })
        })();
        if result.is_err() {
            if let Ok(mut state) = self.state() {
                state.attention = true;
            }
        }
        self.changed();
        result
    }

    pub async fn diagnose(self: &Arc<Self>, id: &str, network: bool) -> Result<()> {
        let (c, mut cancel) = {
            let mut state = self.state()?;
            Self::writable(&state)?;
            if state.diagnostic.is_some() {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "A connection check is already running.",
                ));
            }
            if state
                .last_diagnostic
                .is_some_and(|last| last.elapsed() < COOLDOWN)
            {
                return Err(AppError::new(
                    ErrorCode::RateLimited,
                    "Wait two seconds between connection checks.",
                ));
            }
            let c = Self::find(&state, id)?;
            let (sender, receiver) = watch::channel(false);
            self.record(&mut state, "diagnostics", "requested", Some(id))?;
            state.diagnostic = Some((id.into(), sender));
            state.last_diagnostic = Some(Instant::now());
            (c, receiver)
        };
        let _guard = DiagnosticGuard { core: self.clone() };
        self.changed();
        let report = tokio::select! {
            report = diagnostics::run(&c, &self.home, network) => report,
            _ = cancel.changed() => return Err(AppError::new(ErrorCode::Cancelled, "Connection check cancelled.")),
        };
        let mut state = self.state()?;
        self.record(&mut state, "diagnostics", "completed", Some(id))?;
        if state.config.connections.iter().any(|current| current == &c) {
            state.attention = report
                .checks
                .iter()
                .any(|c| c.status == CheckStatus::Fail || c.status == CheckStatus::Warning);
            state.reports.insert(id.into(), report);
        }
        Ok(())
    }
}

pub struct LaunchLease {
    core: Arc<Core>,
    id: String,
    pub spec: LaunchSpec,
    pub terminal_shell: TerminalShell,
    finished: bool,
}
impl LaunchLease {
    pub fn finish(mut self, result: Result<()>) -> Result<()> {
        {
            let mut state = self.core.state()?;
            state.launching = None;
            state.attention = result.is_err();
            self.core.record(
                &mut state,
                "terminal launch",
                if result.is_ok() {
                    "terminal opened"
                } else {
                    "failed"
                },
                Some(&self.id),
            )?;
        }
        self.finished = true;
        self.core.changed();
        result
    }
}
impl Drop for LaunchLease {
    fn drop(&mut self) {
        if !self.finished {
            if let Ok(mut state) = self.core.state() {
                state.launching = None;
                state.attention = true;
                let _ =
                    self.core
                        .record(&mut state, "terminal launch", "interrupted", Some(&self.id));
            }
            self.core.changed();
        }
    }
}
struct DiagnosticGuard {
    core: Arc<Core>,
}
impl Drop for DiagnosticGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.core.state() {
            state.diagnostic = None;
        }
        self.core.changed();
    }
}

// Import text only: a symlink, pipe, socket, or device must never become an input stream.
fn open_launcher(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > 8192 {
        return Err(std::io::Error::other(
            "The launcher must be a regular file of at most 8 KiB.",
        ));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other("Not a regular file."));
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn sample() -> (tempfile::TempDir, Arc<Core>, String) {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(dir.path().join("app"), dir.path().into());
        core.execute(
            MainRequest::Import {
                name: "Studio".into(),
                command: "ssh dev@studio".into(),
            },
            false,
        )
        .await
        .unwrap();
        let id = core.config().unwrap().favorite_id.unwrap();
        (dir, core, id)
    }
    #[tokio::test]
    async fn favorite_and_launch_lock_are_owned_by_rust_and_survive_restart() {
        let (dir, core, id) = sample().await;
        core.execute(MainRequest::LaunchLock { locked: true }, false)
            .await
            .unwrap();
        assert_eq!(
            core.begin_launch(Some(&id)).err().unwrap().code,
            ErrorCode::Locked
        );
        assert!(!core.view("").unwrap().connections[0].can_connect);
        let reloaded = Core::open(dir.path().join("app"), dir.path().into());
        assert!(reloaded.config().unwrap().launch_locked);
    }
    #[tokio::test]
    async fn dropping_launch_releases_busy_state_but_preserves_cooldown() {
        let (_dir, core, id) = sample().await;
        let lease = core.begin_launch(Some(&id)).unwrap();
        assert_eq!(
            core.begin_launch(Some(&id)).err().unwrap().code,
            ErrorCode::Busy
        );
        drop(lease);
        assert_eq!(
            core.begin_launch(Some(&id)).err().unwrap().code,
            ErrorCode::RateLimited
        );
        assert_eq!(core.pet().unwrap().mood, PetMood::Attention);
    }
    #[tokio::test]
    async fn deletion_reassigns_favorite_and_unknown_ids_cannot_launch() {
        let (_dir, core, id) = sample().await;
        core.execute(MainRequest::Delete { id }, false)
            .await
            .unwrap();
        assert!(core.config().unwrap().favorite_id.is_none());
        assert_eq!(
            core.begin_launch(Some("injected")).err().unwrap().code,
            ErrorCode::NotFound
        );
    }
    #[tokio::test]
    async fn invalid_mutation_does_not_change_saved_data() {
        let (_dir, core, _) = sample().await;
        let draft = ConnectionDraft {
            name: "bad".into(),
            host: "-oProxyCommand=evil".into(),
            ..Default::default()
        };
        assert!(core
            .execute(MainRequest::Save { draft }, false)
            .await
            .is_err());
        assert_eq!(core.config().unwrap().connections.len(), 1);
    }
    #[tokio::test]
    async fn activity_does_not_contain_addresses_commands_or_key_paths() {
        let (_dir, core, _) = sample().await;
        let json = serde_json::to_string(&core.view("").unwrap().activity).unwrap();
        assert!(!json.contains("dev@studio"));
        assert!(!json.contains("ssh "));
        assert!(!json.contains("identityFile"));
    }
}
