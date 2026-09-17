# Droplet

A small water-drop desktop pet that keeps your SSH connections close. Built with **Rust, Tauri 2, TypeScript, and a small native AppKit bridge**.

![Droplet browser preview with sample connection details](artifacts/droplet-preview.png)

## Use it on macOS

Open `Droplet.app`. On its first launch, Droplet imports the plain SSH invocation in `~/Desktop/zbook-studio-ssh.command`, if that file exists. It reads connection details; it never executes the command file. Your original launcher stays intact.

- **Menu bar:** click the drop for a direct connection to your favorite, the connection window, pet controls, or Quit.
- **Desktop pet:** click to open connections, double-click to connect to your favorite, or drag to move it. Its position is saved. The menu includes **Bring pet back to screen** if a monitor layout changes.
- **Connection window:** add, edit, remove, or favorite a connection. You can also paste a plain `ssh` command with `-i` and `-p` options. Hostnames, IP addresses, and SSH config aliases are supported. The saved port is explicitly passed to SSH (22 by default).
- **Touch Bar:** on supported Macs, Droplet provides an app button and a favorite-connection button while a Droplet window is active. This uses public `NSTouchBar` APIs; it is not a persistent Control Strip replacement.
- **Preferences:** opt into launching at login. Closing the connection window leaves Droplet running; **Quit Droplet** in the menu bar exits it. Escape hides the window, and Command-N adds a connection.
- **Quiet movements:** disable pet animation. System reduced-motion preferences are also respected.

On the first connection, macOS may ask you to allow Droplet to control Terminal. Terminal handles passwords, SSH key passphrases, first-time host verification, and connection errors. Droplet reports that Terminal opened; it does not claim to monitor an authenticated session.

If the destination needs a VPN such as Tailscale, that network still needs to be available.

## Develop and build

Requirements: macOS 11+, Xcode Command Line Tools, Node.js 22.12+ (or current LTS), and Rust 1.98+. The first build was verified on Apple Silicon with Rust 1.98.1 and Node.js 26.4.

```sh
# If Rust was installed with rustup and isn't on your PATH:
source "$HOME/.cargo/env"

npm ci
npm run app:dev

# Produce a standalone macOS app:
npm run app:build
```

The app is produced at `src-tauri/target/release/bundle/macos/Droplet.app`. Copy it to `~/Applications` or `/Applications` before enabling launch at login, so its location stays stable.

`npm run dev` runs a **browser preview** using sample connection details. Browser preview mode never opens SSH, changes login items, or reads your real connections.

## Verification

```sh
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

The Playwright suite uses installed Google Chrome. It exercises connection workflows, favorites, cancellation, escaped rendering, pet controls, drag-versus-click behavior, minimum-window layout, and WCAG A/AA automated checks. Rust tests cover strict launcher parsing, option rejection, key-path expansion, shell injection resistance, atomic settings replacement, private file permissions, and corruption handling.

Native checks require an unlocked Mac: inspect the transparent pet and menu icon; move the pet; close and reopen the main window; launch Terminal from a connection; quit/reopen and confirm position/preferences persist. Touch Bar behavior needs supported hardware or Xcode's simulator. Automated accessibility checks do not replace VoiceOver or hardware testing.

## Data and security

Connection details and pet preferences are stored in:

```text
~/Library/Application Support/com.jarrod.droplet/connections.json
```

The file is written atomically with owner-only permissions on macOS. No passwords or private-key contents are stored or read by Droplet. Only key paths are retained. The initial import is specific to the named Desktop launcher; other files are not scanned. There is no telemetry, remote content, or network polling.

The frontend receives narrow Rust commands; it has no generic filesystem or shell plugin. Host and username inputs are validated. Shell arguments are quoted, and the invocation is passed to a fixed AppleScript through argv. Existing SSH configuration and host-key verification stay in effect. Unsupported script bodies, remote commands, and arbitrary SSH flags are rejected by the importer. Advanced SSH behavior belongs in your own SSH config.

If an existing settings file cannot be read or validated, the app displays the error and refuses to overwrite it. Repair or move that file and restart to recover.

## Structure and platform roadmap

| Area | Files |
| --- | --- |
| Connections, preferences, windows, menu bar | `src-tauri/src/lib.rs` |
| Portable connection model and safe import | `src-tauri/src/model.rs` |
| Atomic local persistence | `src-tauri/src/storage.rs` |
| Terminal launch adapter and quoting | `src-tauri/src/terminal.rs` |
| macOS Touch Bar bridge | `src-tauri/src/touchbar.rs`, `src-tauri/native/touchbar.m` |
| Interface, pet, browser preview | `src/main.ts`, `src/art.ts`, `src/bridge.ts`, `src/style.css` |

**macOS is the implemented launch target in 0.1.** The model, storage, Tauri windows, and interface are structured for reuse. Windows and Linux terminal launch adapters are intentionally left for a later version, and have not been built or tested. Non-macOS builds return an explicit unsupported-launch message. Linux window-manager and system-tray differences will need native testing.

The transparent macOS pet uses Tauri's `macos-private-api` feature. This local app is suitable for direct distribution; the current transparent-window implementation is not compatible with Mac App Store review. Public distribution also needs proper signing/notarization. See [Tauri's window configuration](https://v2.tauri.app/reference/config/#windowconfig) and [Apple's Touch Bar documentation](https://developer.apple.com/documentation/appkit/nstouchbar).

## Practical quality decisions

| Quality | Decision and trade-off |
| --- | --- |
| Functional suitability | Three entry points lead to the same saved favorite and Terminal launch operation. Session state stays in Terminal. |
| Reliability | Atomic saves, visible errors, duplicate-launch protection, single-instance behavior, and a pet-position reset. Corrupt settings are preserved. |
| Performance | System webview, small frontend, CSS/SVG animation, and no polling. A second small webview provides the floating pet; quiet mode removes animation. |
| Maintainability | Model, persistence, and terminal adapter are separated; AppKit is isolated behind a macOS build gate. Lockfiles are included. |
| Compatibility | Uses installed Terminal and OpenSSH, including SSH config and agent support. Touch Bar gracefully has no effect on unsupported Macs. |
| Security | No key-content handling; validated input, quoted command arguments, restrictive CSP, and minimal webview permissions. |
| Usability | Favorites, keyboard access, visible errors, reversible pet visibility, destructive-action confirmation, and reduced motion. |
| Portability | Rust and Tauri provide shared foundations. Native terminal adapters and platform validation remain explicit future work. |

In use, the favorite shortcut minimizes steps, the pet can be hidden when distracting, settings survive restarts, and existing SSH safeguards remain intact. Offline or unavailable hosts are handled visibly by Terminal. Multi-monitor recovery is available from the menu bar. Touch Bar controls supplement the mouse and keyboard interface.
