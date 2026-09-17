use crate::{
    model::Connection,
    protocol::{Check, CheckStatus, DiagnosticReport},
    security,
};
use std::{
    net::ToSocketAddrs,
    path::Path,
    time::{Duration, Instant},
};
use tokio::{io::AsyncReadExt, net::TcpStream, sync::Semaphore, time::timeout};

// OS DNS calls cannot be interrupted. Keep the permit inside the blocking job,
// so cancelled checks cannot accumulate unbounded resolver work.
static RESOLVER: Semaphore = Semaphore::const_new(1);

pub async fn run(connection: &Connection, home: &Path, network: bool) -> DiagnosticReport {
    let started = Instant::now();
    let mut checks = vec![Check::new(
        "Launch policy",
        CheckStatus::Pass,
        "Host-key prompts are enforced. Agent/X11 forwarding and LocalCommand are disabled.",
    )];
    match security::check_identity(connection, home) {
        Ok(Some(_)) => checks.push(Check::new("SSH key", CheckStatus::Pass, "The key is a regular, owner-only file. Its contents were not read.")),
        Ok(None) => checks.push(Check::new("SSH key", CheckStatus::Info, "No explicit key selected. OpenSSH will use your agent/config; those keys are not inspected here.")),
        Err(error) => checks.push(Check::new("SSH key", CheckStatus::Fail, error.message)),
    }
    if network {
        match timeout(Duration::from_secs(5), network_checks(connection)).await {
            Ok(results) => checks.extend(results),
            Err(_) => checks.push(Check::new("Network", CheckStatus::Warning, "The five-second diagnostic budget expired. A VPN, firewall, or slow DNS may be involved.")),
        }
    } else {
        checks.push(Check::new(
            "Network",
            CheckStatus::Info,
            "Not requested. No network connection was made.",
        ));
    }
    let summary = if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        "Needs attention"
    } else if checks.iter().any(|c| c.status == CheckStatus::Warning) {
        "Some checks need a look"
    } else {
        "Checks completed"
    }
    .into();
    DiagnosticReport {
        connection_id: connection.id.clone(),
        summary,
        checks,
        elapsed_ms: started.elapsed().as_millis().min(u32::MAX as u128) as u32,
    }
}

async fn network_checks(connection: &Connection) -> Vec<Check> {
    let host = connection
        .host
        .trim_start_matches('[')
        .trim_end_matches(']');
    // Deliberately do not run ssh -G: reading SSH config can execute Match exec hooks.
    // These are direct checks of the saved host/port, not ProxyJump or SSH alias checks.
    let permit = RESOLVER
        .acquire()
        .await
        .expect("resolver semaphore is never closed");
    let host = host.to_owned();
    let port = connection.port;
    let resolved = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        (host.as_str(), port)
            .to_socket_addrs()
            .map(|addresses| addresses.take(8).collect::<Vec<_>>())
    })
    .await;
    let addresses = match resolved {
        Ok(Ok(addresses)) => addresses,
        _ => return vec![Check::new("DNS", CheckStatus::Warning, "The saved host did not resolve. SSH config aliases and jump hosts are not evaluated by this direct check.")],
    };
    if addresses.is_empty() {
        return vec![Check::new(
            "DNS",
            CheckStatus::Warning,
            "No address was returned for the saved host.",
        )];
    }
    let mut checks = vec![Check::new(
        "DNS",
        CheckStatus::Pass,
        "The saved host resolved. At most eight addresses are checked.",
    )];
    let mut connected = None;
    for address in addresses {
        if let Ok(Ok(stream)) =
            timeout(Duration::from_millis(450), TcpStream::connect(address)).await
        {
            connected = Some(stream);
            break;
        }
    }
    let Some(mut stream) = connected else {
        checks.push(Check::new("TCP port", CheckStatus::Warning, "The direct port was not reachable within the check budget. Your SSH configuration may use a different route."));
        return checks;
    };
    checks.push(Check::new("TCP port", CheckStatus::Pass, "The direct port accepted a connection. This does not establish server identity or authenticate SSH."));
    let mut banner = [0u8; 1024];
    let read = timeout(Duration::from_millis(700), async {
        let mut used = 0;
        while used < banner.len() {
            let count = stream.read(&mut banner[used..]).await?;
            if count == 0 {
                break;
            }
            used += count;
            if banner[..used]
                .split(|b| *b == b'\n')
                .any(|line| line.starts_with(b"SSH-2.0-"))
            {
                return Ok::<_, std::io::Error>(true);
            }
        }
        Ok(false)
    })
    .await;
    checks.push(if matches!(read, Ok(Ok(true))) {
        Check::new("SSH greeting", CheckStatus::Pass, "An SSH 2.0 greeting was received. The server key and login have not been verified.")
    } else { Check::new("SSH greeting", CheckStatus::Warning, "No SSH 2.0 greeting arrived within 700 ms. The port may be another service or respond slowly.") });
    checks
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    #[tokio::test]
    async fn local_ssh_greeting_is_not_reported_as_authenticated() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut c = crate::model::parse_launcher("ssh dev@127.0.0.1", "Test").unwrap();
        c.port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            stream.write_all(b"SSH-2.0-test\r\n").await.unwrap();
        });
        let report = run(&c, Path::new("/tmp"), true).await;
        assert!(report
            .checks
            .iter()
            .any(|c| c.label == "SSH greeting" && c.status == CheckStatus::Pass));
        assert!(report
            .checks
            .iter()
            .any(|c| c.message.contains("not been verified")));
        server.await.unwrap();
    }
    #[tokio::test]
    async fn local_checks_do_not_attempt_dns_or_connections() {
        let c = crate::model::parse_launcher("ssh no-such-host.invalid", "Test").unwrap();
        let report = run(&c, Path::new("/tmp"), false).await;
        assert!(!report.checks.iter().any(|c| c.label == "DNS"));
    }
}
