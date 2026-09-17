# Security boundaries

Droplet 0.2 is a local macOS SSH launcher. It delegates authentication to the installed OpenSSH client in Terminal. It never loads private-key contents or stores passwords. Its direct connection checks are unauthenticated and opt-in.

## Rust owns the decisions

`droplet-core` owns connection validation, import parsing, search, favorites, persistence, launch policy, launch locking, cooldowns, diagnostics, and activity history. Tauri owns OS integration. React TSX renders Rust view models and submits typed user requests; it only keeps transient presentation state such as form text, open dialogs, and pointer gestures. Rust generates the TypeScript contracts, and `npm run types:check` checks for drift.

A launch accepts only the ID of an existing connection. The pet can request the current favorite without supplying an ID. No IPC command accepts an executable, shell script, arbitrary SSH options, or a raw connection to launch. Plain SSH imports are parsed as data and must pass the same model validation.

## Webview isolation

- The main window and pet have separate Tauri capabilities. The build manifest declares both custom commands so Tauri's runtime authority applies to them.
- Rust checks the calling window label again. Pet requests cannot change settings, edit connections, or resume paused launches.
- The pet receives its name/status/animation view, without addresses, key paths, configuration, or history. Global change events contain no data. Errors from native shortcuts target the main window.
- Neither renderer has generic file, process, HTTP, or shell permissions. The pet has no generic window-position permission; Rust restricts moves to its own window and available display bounds.
- The production CSP limits scripts and styles to local resources and connections to Tauri IPC. Remote navigation is rejected. React renders connection text without raw HTML injection.

A compromised **main** renderer can still perform its authorized app operations, including editing connections and requesting launches. A compromised pet can request the favorite while launches are enabled. These boundaries reduce available authority; they do not require proof of a human click.

## OpenSSH policy

Each launch supplies these options before the destination:

| Option | Behavior |
| --- | --- |
| `StrictHostKeyChecking=ask` | Keep first-use confirmation and reject changed host keys. |
| `UserKnownHostsFile` | Use the user's `.ssh/known_hosts`, instead of a per-host config redirect. System host files still apply. |
| `ForwardAgent=no`, `ForwardX11=no` | Prevent implicit agent and X11 forwarding. |
| `PermitLocalCommand=no` | Disable the SSH `LocalCommand` hook. |
| `ClearAllForwardings=yes` | Disable configured local, remote, and dynamic port forwards for this launch. |
| `RemoteCommand=none` | Open a normal session instead of a configured remote command. |
| `ConnectTimeout=10`, `ConnectionAttempts=1` | Bound the connection attempt. |
| `ServerAliveInterval=30`, `ServerAliveCountMax=3` | Let OpenSSH detect an unresponsive session. |
| `IdentitiesOnly=yes` with an explicit key | Limit agent identities; identities configured in SSH config can still apply. |

Host/username syntax is constrained; paths are passed as quoted literal shell words through fixed AppleScript argv. Explicit key files must be regular, owned by the current user, and have no group/other permission bits on Unix. Final key symlinks and OpenSSH path-expansion tokens are rejected. Droplet does not silently chmod files. Files selected indirectly by SSH config or an agent are handled by OpenSSH and are not inspected by Droplet.

**Your SSH configuration is trusted local code.** `ProxyCommand`, `ProxyJump`, `Match exec`, `KnownHostsCommand`, certificate authorities, shell startup files, and installed binaries remain outside Droplet's protection. This is not an SSH sandbox or a replacement for reviewing your config. Command-line options intentionally override some existing SSH behavior, so connections that depend on forwarding or remote commands should be launched separately in Terminal. Droplet does not modify your SSH configuration.

Key checks happen before handing the pathname to OpenSSH. They do not eliminate same-user replacement races, parent-directory symlink races, or extra macOS ACL grants. A user/root process that can change Droplet's data, keys, binaries, or trusted configuration is outside the threat boundary.

## Reliability and resource limits

- One Terminal-opening operation at a time; a two-second launch cooldown. A Rust drop guard releases busy state even if the task is cancelled. The AppleScript process has a 35-second deadline and is killed on cancellation. An already-open Terminal session is not terminated.
- The persistent launch pause blocks new requests from all four entry points. It is not authentication and does not stop existing sessions or requests already accepted.
- One diagnostic operation at a time, with a two-second cooldown and cancellation. Each direct network check has a five-second overall budget, up to eight resolved addresses, 450 ms per TCP attempt, and 700 ms / 1 KiB for an SSH greeting. Diagnostics do not send credentials, display remote banner contents, or prove server identity.
- System DNS calls are not interruptible. A semaphore held by the blocking resolver task limits them to one per process, even after cancellation. The UI can stop waiting while that OS call completes.
- Diagnostics inspect the saved host/port directly. They do not evaluate SSH aliases, proxies, or `ssh -G`, because parsing user configuration can execute `Match exec`. A direct check can fail while an SSH alias or proxy connection works.
- Editing a connection during a check causes the old result to be discarded.

## Local data and activity

Connection settings and the last 200 activity entries are stored under `~/Library/Application Support/com.jarrod.droplet`. Writes use an owner-only temporary file, sync its contents, and atomically replace the destination. Reads are bounded at 512 KiB; special files and final-file symlinks are refused. New data directories use mode 700; an existing owned directory must not be writable by other users. Data files require owner-only permissions on Unix.

Invalid data is preserved and blocks connection mutations/launches until repaired and the app restarted. Atomic replacement avoids partially written JSON; this is not a database transaction across settings and history or a guarantee against every power-loss scenario. Only the desktop app writes live data; the CLI's `list` and `doctor` read it without taking ownership of state.

History records an event ID, timestamp, action, outcome, and optional connection UUID. It omits hosts, usernames, key paths, command strings, diagnostic output, and secrets. Settings operations record **requested** before the write, so an interrupted write is not falsely called successful. Terminal results mean **Terminal opened**, not **SSH authenticated**. History is neither encrypted nor tamper-proof, and a process running as the same user can change it.

## Development and distribution

The browser preview is read-only static data generated by Rust. Playwright's development-only transport runs the real core in a temporary directory with sample data, no launch adapter, and network diagnostics disabled. That transport hook is removed from the production JS bundle.

Windows and Linux launch adapters are not implemented or tested. Unix mode checks do not establish Windows ACL security; a Windows adapter must add ACL validation before claiming equivalent protections.

The local macOS bundle is ad-hoc signed, not notarized. Public distribution needs Developer ID signing, notarization, update signing, and platform testing. The transparent pet uses Tauri's macOS private API feature and is not currently suited to Mac App Store submission.

References: [Tauri runtime authority](https://v2.tauri.app/security/runtime-authority/), [Tauri app command manifest](https://docs.rs/tauri-build/latest/tauri_build/struct.AppManifest.html), [OpenSSH configuration semantics](https://man.openbsd.org/ssh_config).
