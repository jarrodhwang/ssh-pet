use droplet_core::{
    model::{self, Config},
    protocol::{ConnectionDraft, MainRequest, PetRequest},
    security, storage, Core, ErrorCode,
};
use std::path::Path;

#[test]
fn protocol_rejects_undeclared_actions_and_fields() {
    assert!(serde_json::from_str::<MainRequest>(
        r#"{"type":"connect","id":"x","command":"touch /tmp/injected"}"#
    )
    .is_err());
    assert!(serde_json::from_str::<PetRequest>(r#"{"type":"save","draft":{}}"#).is_err());
    assert!(serde_json::from_str::<PetRequest>(r#"{"type":"launchLock","locked":false}"#).is_err());
}

#[test]
fn old_settings_get_a_safe_default_for_the_new_launch_lock() {
    let config: Config = serde_json::from_str(r#"{"version":1,"connections":[],"favoriteId":null,"petVisible":true,"reduceMotion":false,"petPosition":null}"#).unwrap();
    assert!(!config.launch_locked);
    config.validate().unwrap();
}

#[test]
fn expansion_tokens_cannot_bypass_explicit_key_metadata_checks() {
    for name in ["/tmp/%d/key", "/tmp/${HOME}/key"] {
        let mut c = model::parse_launcher("ssh studio", "Studio").unwrap();
        c.identity_file = name.into();
        assert_eq!(
            security::check_identity(&c, Path::new("/tmp"))
                .unwrap_err()
                .code,
            ErrorCode::UnsafeKey
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn installed_openssh_accepts_and_enforces_the_generated_policy() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("ssh_config");
    std::fs::write(&config, "Host *\n StrictHostKeyChecking no\n ForwardAgent yes\n ForwardX11 yes\n PermitLocalCommand yes\n RemoteCommand echo should-not-run\n LocalForward 12345 localhost:22\n").unwrap();
    let c = model::parse_launcher("ssh dev@127.0.0.1", "Local test").unwrap();
    let spec = security::launch_spec(&c, directory.path()).unwrap();
    // -G only with our controlled config. No real user config or network access.
    let output = std::process::Command::new(spec.program)
        .args(["-G", "-F"])
        .arg(config)
        .args(spec.args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let policy = String::from_utf8(output.stdout).unwrap();
    for setting in [
        "stricthostkeychecking ask",
        "forwardagent no",
        "forwardx11 no",
        "permitlocalcommand no",
        "clearallforwardings yes",
        "serveraliveinterval 30",
        "connecttimeout 10",
    ] {
        assert!(
            policy.lines().any(|line| line == setting),
            "Missing: {setting}"
        );
    }
    assert!(!policy
        .lines()
        .any(|line| line.starts_with("remotecommand ")));
    assert!(!policy.lines().any(|line| line.starts_with("localforward ")));
    assert!(policy.contains(&format!(
        "userknownhostsfile {}",
        directory.path().join(".ssh/known_hosts").display()
    )));
}

#[tokio::test]
async fn corrupt_data_blocks_mutations_and_launches_without_overwriting() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    storage::save(&data.join("connections.json"), &Config::default()).unwrap();
    std::fs::write(data.join("connections.json"), "broken").unwrap();
    let core = Core::open(data.clone(), temp.path().into());
    assert!(core.view("").unwrap().load_error.is_some());
    assert!(core
        .execute(MainRequest::LaunchLock { locked: true }, false)
        .await
        .is_err());
    assert!(core.begin_launch(None).is_err());
    assert_eq!(
        std::fs::read_to_string(data.join("connections.json")).unwrap(),
        "broken"
    );
}

async fn network_sample() -> (
    tempfile::TempDir,
    std::sync::Arc<Core>,
    String,
    tokio::net::TcpListener,
) {
    let temp = tempfile::tempdir().unwrap();
    let core = Core::open(temp.path().join("data"), temp.path().into());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    core.execute(
        MainRequest::Import {
            name: "Local test".into(),
            command: format!(
                "ssh -p {} dev@127.0.0.1",
                listener.local_addr().unwrap().port()
            ),
        },
        false,
    )
    .await
    .unwrap();
    let id = core.config().unwrap().favorite_id.unwrap();
    (temp, core, id, listener)
}

#[tokio::test]
async fn cancelled_diagnostics_release_busy_state_and_do_not_publish_results() {
    let (_temp, core, id, listener) = network_sample().await;
    let run_core = core.clone();
    let run_id = id.clone();
    let task = tokio::spawn(async move { run_core.diagnose(&run_id, true).await });
    let (_stream, _) = tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept())
        .await
        .unwrap()
        .unwrap();
    assert!(core.view("").unwrap().diagnostic_running);
    assert_eq!(
        core.diagnose(&id, true).await.unwrap_err().code,
        ErrorCode::Busy
    );
    core.execute(MainRequest::CancelDiagnostics, false)
        .await
        .unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.unwrap_err().code, ErrorCode::Cancelled);
    let view = core.view("").unwrap();
    assert!(!view.diagnostic_running);
    assert!(view.connections[0].report.is_none());
}

#[tokio::test]
async fn results_from_an_edited_connection_are_discarded() {
    let (_temp, core, id, listener) = network_sample().await;
    let run_core = core.clone();
    let task = tokio::spawn(async move { run_core.diagnose(&id, true).await });
    let (mut stream, _) =
        tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
    let mut draft: ConnectionDraft = (&core.config().unwrap().connections[0]).into();
    draft.host = "different.invalid".into();
    core.execute(MainRequest::Save { draft }, false)
        .await
        .unwrap();
    use tokio::io::AsyncWriteExt;
    stream.write_all(b"SSH-2.0-test\r\n").await.unwrap();
    task.await.unwrap().unwrap();
    assert!(core.view("").unwrap().connections[0].report.is_none());
}

#[tokio::test]
async fn stalled_banner_has_a_short_deadline() {
    let (_temp, core, id, listener) = network_sample().await;
    let started = std::time::Instant::now();
    let run_core = core.clone();
    let task = tokio::spawn(async move { run_core.diagnose(&id, true).await });
    let (_stream, _) = tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    let view = core.view("").unwrap();
    assert!(view.connections[0]
        .report
        .as_ref()
        .unwrap()
        .checks
        .iter()
        .any(|c| c.label == "SSH greeting"
            && c.status == droplet_core::protocol::CheckStatus::Warning));
}
