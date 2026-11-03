//! Persistent programs on a dedicated tmux server. Mobux itself stays outside tmux.
//! Graphics passthrough and nested child multiplexers are deliberately not configured.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::str::FromStr;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

const OWNER: &str = "mobux-v1";
const RECORD_FORMAT: &str = "#{session_name}\t#{@mobux-owner}\t#{@mobux-ready}\t#{@mobux-spec}\t#{@mobux-pane}\t#{pane_id}\t#{pane_dead}\t#{pane_dead_status}";
// Static shell code only: cwd, binary, and argv are separate positional arguments.
// tmux's -c may fall back to HOME; this guard must succeed before the exec.
const EXEC_GUARD: &str = "cd -P \"$1\" || exit 125; shift; exec nice -n 0 -- \"$@\"";
// Each half, including its padding, is a target on BOTH status rows. These are
// tmux user ranges, not Ratatui events. The explicit + avoids the modifier
// parser mistaking the following '#' for a delimiter; l makes labels literal.
const FOOTER: &str = "#[range=user|home]#{p+#{e|/:#{client_width},2};l: Home }#[range=user|apps]#{p-#{e|-:#{client_width},#{e|/:#{client_width},2}};l: Apps }#[norange]";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchSpec {
    pub title: String,
    pub bin: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// An opaque, generation-specific tmux session name, never a numeric ID or target expression.
#[derive(Clone, Debug, Serialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn target(&self) -> String {
        // The colon also disambiguates set-option's window/pane target parser.
        format!("={self}:")
    }
}

impl FromStr for SessionId {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let valid = value.strip_prefix("mobux-").is_some_and(|suffix| {
            Uuid::parse_str(suffix).is_ok_and(|uuid| {
                uuid.get_version_num() == 4
                    && uuid.get_variant() == uuid::Variant::RFC4122
                    && uuid.to_string() == suffix
            })
        });
        if valid {
            Ok(Self(value.to_owned()))
        } else {
            Err("Invalid Mobux session ID: expected 'mobux-' followed by a canonical UUIDv4".into())
        }
    }
}

impl TryFrom<&str> for SessionId {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedSession {
    pub id: SessionId,
    pub spec: LaunchSpec,
    pub status: SessionStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionStatus {
    Running,
    Exited(Option<i32>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnTo {
    Home,
    Apps,
}

// Explicit unlock matters when another thread forks a command while this file
// is open: a transient inherited descriptor must not retain a dropped lock.
struct SessionLock(File);

impl Drop for SessionLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

#[derive(Clone, Debug)]
pub struct Backend {
    socket: String,
    lock_path: Option<PathBuf>,
}

impl Default for Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend {
    /// Checks the minimum tmux feature version, without starting a server.
    /// Running Mobux within any tmux instance is unsupported.
    pub fn available() -> bool {
        if nested() {
            return false;
        }
        Self::new()
            .command()
            .arg("-V")
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && supported_version(&String::from_utf8_lossy(&output.stdout))
            })
    }

    pub fn new() -> Self {
        Self::with_socket("mobux").expect("default socket name is valid")
    }

    /// A separate namespace for tests or independent Mobux workspaces.
    pub fn with_socket(socket: &str) -> Result<Self, String> {
        if socket.is_empty()
            || socket.len() > 64
            || !socket
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'-' | b'_'))
        {
            return Err(
                "Session socket must contain 1 to 64 letters, digits, hyphens, or underscores"
                    .to_string(),
            );
        }
        let cache = cache_directory(std::env::var_os("XDG_CACHE_HOME"), std::env::var_os("HOME"));
        Ok(Self {
            socket: socket.to_string(),
            lock_path: cache.map(|cache| cache.join("mobux").join(format!("{socket}.lock"))),
        })
    }

    /// Lists all ready records owned by Mobux; there is no UI-sized limit.
    pub fn list(&self) -> Result<Vec<ManagedSession>, String> {
        self.check_context()?;
        let output = self
            .command()
            .args(["list-panes", "-a", "-F", RECORD_FORMAT])
            .output()
            .map_err(|error| format!("Cannot run tmux: {error}"))?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            if message.contains("no server running")
                || (message.contains("error connecting to")
                    && (message.contains("No such file or directory")
                        || message.contains("Connection refused")))
            {
                return Ok(Vec::new());
            }
            return Err(command_error(&output));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(parse_record)
            .collect())
    }

    /// Reuses the exact (binary, argv, canonical directory) identity, including
    /// exited sessions. Title changes never start or restart a program.
    pub fn ensure(&self, spec: &LaunchSpec) -> Result<ManagedSession, String> {
        self.check_context()?;
        let spec = canonical_spec(spec)?;
        let _lock = self.lock()?;
        if let Some(session) = self
            .list()?
            .into_iter()
            .find(|session| same_identity(&session.spec, &spec))
        {
            return Ok(session);
        }
        self.create(spec)
    }

    /// The caller must restore its terminal before calling this blocking method.
    /// Detaching or quitting Mobux never kills the persistent program.
    pub fn attach(&self, session: &ManagedSession) -> Result<ReturnTo, String> {
        self.attach_with(session, || Ok(()))
    }

    /// Run preflight while the workspace is still visible; hand over the TTY only
    /// immediately before spawning the client. Failed validation never blanks it.
    pub fn attach_with(
        &self,
        session: &ManagedSession,
        handoff: impl FnOnce() -> Result<(), String>,
    ) -> Result<ReturnTo, String> {
        self.check_context()?;
        let current = self.owned(&session.id)?;
        if current.status != SessionStatus::Running {
            return Err("This session has exited. Restart it explicitly to run it again.".into());
        }
        self.run(&[
            "set-option",
            "-t",
            &current.id.target(),
            "@mobux-return",
            "home",
        ])?;
        // The queued dead-pane check closes the exit-before-attach race; the
        // pane-died hook handles exits after this check. Neither respawns.
        handoff()?;
        let status = self
            .command()
            .args([
                "attach-session",
                "-t",
                &current.id.target(),
                ";",
                "if-shell",
                "-F",
                "#{pane_dead}",
                "detach-client",
            ])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|error| format!("Cannot attach tmux client: {error}"))?;
        if !status.success() {
            return Err(format!("tmux attach failed ({status})"));
        }
        let output = self.run(&[
            "show-options",
            "-qv",
            "-t",
            &current.id.target(),
            "@mobux-return",
        ]);
        Ok(match output {
            Ok(value) if value.trim() == "apps" => ReturnTo::Apps,
            _ => ReturnTo::Home,
        })
    }

    pub fn restart(&self, id: &SessionId) -> Result<ManagedSession, String> {
        self.check_context()?;
        let _lock = self.lock()?;
        let session = self.owned(id)?;
        if session.status == SessionStatus::Running {
            return Err("Session is still running; restart will not kill it.".into());
        }
        let spec = canonical_spec(&session.spec)?;
        self.respawn(id, &spec)?;
        self.owned(id)
    }

    /// Only explicit, UI-confirmed stop should call this method.
    pub fn stop(&self, id: &SessionId) -> Result<(), String> {
        self.check_context()?;
        let _lock = self.lock()?;
        self.owned(id)?;
        self.run(&["kill-session", "-t", &id.target()])?;
        Ok(())
    }

    fn command(&self) -> Command {
        let mut command = Command::new("tmux");
        command.args(["-L", &self.socket, "-f", "/dev/null"]);
        command
    }

    fn check_context(&self) -> Result<(), String> {
        if nested() {
            Err("Mobux must run outside tmux. Leave the enclosing tmux session first.".into())
        } else {
            Ok(())
        }
    }

    fn run(&self, args: &[&str]) -> Result<String, String> {
        let output = self
            .command()
            .args(args)
            .output()
            .map_err(|error| format!("Cannot run tmux: {error}"))?;
        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|error| format!("Invalid tmux response: {error}"))
        } else {
            Err(command_error(&output))
        }
    }

    fn lock(&self) -> Result<SessionLock, String> {
        let path = self
            .lock_path
            .as_ref()
            .ok_or("Set XDG_CACHE_HOME or HOME to store the Mobux session lock.")?;
        let directory = path.parent().ok_or("Invalid Mobux lock directory")?;
        fs::create_dir_all(directory).map_err(|error| format!("Create lock directory: {error}"))?;
        if fs::symlink_metadata(directory)
            .map_err(|error| format!("Inspect lock directory: {error}"))?
            .file_type()
            .is_symlink()
        {
            return Err("Mobux lock directory must not be a symlink".into());
        }
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Protect lock directory: {error}"))?;
        match fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
                return Err("Mobux lock must be a regular file, not a symlink".into());
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(format!("Inspect session lock: {error}"));
            }
            _ => {}
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|error| format!("Open session lock: {error}"))?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("Protect session lock: {error}"))?;
        file.try_lock()
            .map_err(|error| format!("Mobux session backend is busy; try again. ({error})"))?;
        Ok(SessionLock(file))
    }

    fn owned(&self, id: &SessionId) -> Result<ManagedSession, String> {
        self.list()?
            .into_iter()
            .find(|session| &session.id == id)
            .ok_or_else(|| format!("No ready Mobux session {id}"))
    }

    fn create(&self, spec: LaunchSpec) -> Result<ManagedSession, String> {
        let id = SessionId(format!("mobux-{}", Uuid::new_v4()));
        self.create_named(spec, id)
    }

    fn create_named(&self, spec: LaunchSpec, id: SessionId) -> Result<ManagedSession, String> {
        if !Self::available() {
            return Err("Mobux requires tmux 3.6 or newer for reliable exit handling.".into());
        }
        let encoded = STANDARD.encode(
            serde_json::to_vec(&spec).map_err(|error| format!("Encode launch spec: {error}"))?,
        );
        // No -A: even a UUID collision must fail, never reuse an existing name.
        // Start its default shell as a disposable placeholder, then set up
        // remain-on-exit before launching even an immediately exiting program.
        let output = self
            .command()
            .args([
                "new-session",
                "-d",
                "-P",
                "-F",
                "#{session_name}\t#{pane_id}",
                "-s",
                id.as_str(),
                "-c",
            ])
            .arg(cwd_argument(&spec.cwd))
            .output()
            .map_err(|error| format!("Create tmux session: {error}"))?;
        if !output.status.success() {
            return Err(command_error(&output));
        }
        let result = (|| {
            let response = String::from_utf8_lossy(&output.stdout);
            let (created, pane) = response
                .trim()
                .split_once('\t')
                .ok_or("Missing new session name")?;
            if created != id.as_str() {
                return Err("Unexpected new session name".into());
            }
            self.run(&[
                "set-option",
                "-w",
                "-t",
                &id.target(),
                "remain-on-exit",
                "on",
            ])?;
            self.configure(&id)?;
            for (key, value) in [
                ("@mobux-owner", OWNER),
                ("@mobux-spec", encoded.as_str()),
                ("@mobux-pane", pane),
                ("@mobux-return", "home"),
                ("@mobux-ready", "1"),
            ] {
                self.run(&["set-option", "-t", &id.target(), key, value])?;
            }
            self.respawn(&id, &spec)?;
            self.owned(&id)
        })();
        if result.is_err() {
            // Never clean up another session or kill the server on failure.
            let _ = self.run(&["kill-session", "-t", &id.target()]);
        }
        result
    }

    fn configure(&self, id: &SessionId) -> Result<(), String> {
        for (key, value) in [
            ("mouse", "on"),
            ("status", "2"),
            ("status-position", "bottom"),
            ("status-style", "bg=colour236,fg=white,bold"),
            ("status-format[0]", FOOTER),
            ("status-format[1]", FOOTER),
        ] {
            self.run(&["set-option", "-t", &id.target(), key, value])?;
        }
        // These are tmux command lists, never shell commands. Binding context
        // selects precisely the clicking client's session; detach-client with
        // no -a/-s detaches only that client. Other private-server sessions do
        // not get navigation behavior unless they carry our ownership marker.
        self.run(&[
            "bind-key", "-n", "MouseUp1Status", "if-shell", "-F",
            "#{==:#{@mobux-owner},mobux-v1}",
            "if-shell -F '#{==:#{mouse_status_range},home}' 'set-option @mobux-return home ; detach-client' \"if-shell -F '#{==:#{mouse_status_range},apps}' 'set-option @mobux-return apps ; detach-client'\"",
        ])?;
        self.run(&[
            "set-hook",
            "-t",
            &id.target(),
            "pane-died",
            &format!("detach-client -s '{}'", id.target()),
        ])?;
        Ok(())
    }

    fn respawn(&self, id: &SessionId, spec: &LaunchSpec) -> Result<(), String> {
        // A numeric pane ID alone can be reused after a server restart. Resolve
        // it only within this exact generation's session, never globally.
        let pane = self.run(&["show-options", "-qv", "-t", &id.target(), "@mobux-pane"])?;
        let pane = pane.trim();
        if !pane.strip_prefix('%').is_some_and(|digits| {
            !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
        }) {
            return Err("Invalid managed pane ID".into());
        }
        let output = self
            .command()
            .args(["respawn-pane", "-k", "-t", &format!("={id}:.{pane}"), "-c"])
            .arg(cwd_argument(&spec.cwd))
            .arg("--")
            .arg(crate::host::posix_shell())
            .args(["-c", EXEC_GUARD, "mobux-exec"])
            .arg(tmux_argument(&spec.cwd))
            .arg(tmux_argument(&spec.bin))
            .args(spec.args.iter().map(tmux_argument))
            .output()
            .map_err(|error| format!("Launch session program: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(command_error(&output))
        }
    }
}

// Even argv-based tmux commands interpret a final ';' as a command separator.
// Escape exactly that byte for tmux's argv parser, not for any shell.
fn tmux_argument(value: impl AsRef<OsStr>) -> OsString {
    let mut bytes = value.as_ref().as_bytes().to_vec();
    if bytes.last() == Some(&b';') {
        bytes.insert(bytes.len() - 1, b'\\');
    }
    OsString::from_vec(bytes)
}

fn cwd_argument(path: &Path) -> OsString {
    // tmux additionally expands -c as a format. Preserve literal hashes, even
    // '#(command)' or '#{variable}', in real directory names.
    let mut bytes = Vec::new();
    for &byte in path.as_os_str().as_bytes() {
        bytes.push(byte);
        if byte == b'#' {
            bytes.push(byte);
        }
    }
    tmux_argument(OsString::from_vec(bytes))
}

fn cache_directory(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    xdg.map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            home.map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|path| path.join(".cache"))
        })
}

fn nested() -> bool {
    std::env::var_os("TMUX").is_some()
}

fn supported_version(value: &str) -> bool {
    let Some(version) = value.trim().strip_prefix("tmux ") else {
        return false;
    };
    let Some((major, minor)) = version.split_once('.') else {
        return false;
    };
    let minor: String = minor.chars().take_while(char::is_ascii_digit).collect();
    match (major.parse::<u32>(), minor.parse::<u32>()) {
        (Ok(major), Ok(minor)) => (major, minor) >= (3, 6),
        _ => false,
    }
}

fn canonical_spec(spec: &LaunchSpec) -> Result<LaunchSpec, String> {
    if spec.bin.is_empty()
        || spec.bin.contains('\0')
        || spec.args.iter().any(|arg| arg.contains('\0'))
    {
        return Err("Program must be nonempty; program and arguments cannot contain NUL.".into());
    }
    let mut canonical = spec.clone();
    canonical.cwd = fs::canonicalize(&spec.cwd).map_err(|error| {
        format!(
            "Cannot open launch directory {}: {error}",
            spec.cwd.display()
        )
    })?;
    if !canonical.cwd.is_dir() {
        return Err("Launch directory is not a directory".into());
    }
    Ok(canonical)
}

fn same_identity(left: &LaunchSpec, right: &LaunchSpec) -> bool {
    left.bin == right.bin && left.args == right.args && left.cwd == right.cwd
}

fn parse_record(line: &str) -> Option<ManagedSession> {
    let fields: Vec<_> = line.split('\t').collect();
    if fields.len() != 8 || fields[1] != OWNER || fields[2] != "1" || fields[4] != fields[5] {
        return None;
    }
    let id = fields[0].parse().ok()?;
    let spec: LaunchSpec = serde_json::from_slice(&STANDARD.decode(fields[3]).ok()?).ok()?;
    let status = match fields[6] {
        "0" => SessionStatus::Running,
        "1" => SessionStatus::Exited(fields[7].parse().ok()),
        _ => return None,
    };
    Some(ManagedSession { id, spec, status })
}

fn command_error(output: &Output) -> String {
    format!(
        "tmux failed ({}): {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Child;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestServer {
        backend: Backend,
        directory: PathBuf,
    }

    impl TestServer {
        fn new() -> Option<Self> {
            if !Backend::available() {
                assert!(
                    std::env::var_os("CI").is_none(),
                    "CI requires tmux >= 3.6, outside TMUX"
                );
                eprintln!("SKIP tmux integration: tmux >= 3.6 required, outside TMUX");
                return None;
            }
            let socket = format!(
                "mobux-test-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let directory = std::env::temp_dir().join(&socket);
            fs::create_dir_all(&directory).unwrap();
            Some(Self {
                backend: Backend {
                    socket,
                    lock_path: Some(directory.join("cache/sessions.lock")),
                },
                directory,
            })
        }

        fn spec(&self, script: &str) -> LaunchSpec {
            LaunchSpec {
                title: "test".into(),
                bin: "/bin/sh".into(),
                args: vec!["-c".into(), script.into()],
                cwd: self.directory.clone(),
            }
        }

        fn exited(&self, id: &SessionId) -> ManagedSession {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let session = self.backend.owned(id).unwrap();
                // These fixtures exit with numeric codes. tmux can publish
                // pane_dead before its child-status notification is processed.
                if matches!(session.status, SessionStatus::Exited(Some(_))) {
                    return session;
                }
                assert!(
                    Instant::now() < deadline,
                    "program exit code was not published"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        fn pid(&self, id: &SessionId) -> String {
            self.backend
                .run(&["display-message", "-p", "-t", id.as_str(), "#{pane_pid}"])
                .unwrap()
        }
    }

    struct TestClient(Child);

    impl TestClient {
        fn attach(server: &TestServer, id: &SessionId) -> Self {
            Self(
                server
                    .backend
                    .command()
                    .args(["-C", "attach-session", "-t", id.as_str()])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap(),
            )
        }

        fn disconnected(&mut self) {
            let deadline = Instant::now() + Duration::from_secs(5);
            while self.0.try_wait().unwrap().is_none() {
                assert!(Instant::now() < deadline, "client did not detach");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    impl Drop for TestClient {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            // Only this test's unique socket, never mobux or the default server.
            let _ = self.backend.command().arg("kill-server").output();
            let _ = fs::remove_dir_all(&self.directory);
        }
    }

    #[test]
    fn isolated_socket_names_cannot_escape_the_tmux_namespace() {
        for invalid in [
            "",
            "../default",
            "/tmp/default",
            "socket/name",
            "white space",
            "name;kill-server",
        ] {
            assert!(Backend::with_socket(invalid).is_err());
        }
        let own = Backend::with_socket("mobux-e2e-123").unwrap();
        assert_eq!(own.socket, "mobux-e2e-123");
        assert!(own.lock_path.unwrap().ends_with("mobux/mobux-e2e-123.lock"));
    }

    #[test]
    fn strict_ids_and_versions() {
        for good in [
            "mobux-12345678-1234-4234-9234-123456789abc",
            "mobux-ffffffff-ffff-4fff-bfff-ffffffffffff",
        ] {
            let id: SessionId = good.parse().unwrap();
            assert_eq!(id.as_str(), good);
            assert_eq!(id.to_string(), good);
            assert_eq!(
                serde_json::from_str::<SessionId>(&serde_json::to_string(&id).unwrap()).unwrap(),
                id
            );
        }
        for bad in [
            "",
            "$0",
            "$123",
            "0",
            "%0",
            "mobux-",
            "mobux-00000000-0000-0000-0000-000000000000",
            "mobux-12345678-1234-1234-9234-123456789abc",
            "mobux-12345678-1234-4234-1234-123456789abc",
            "mobux-12345678-1234-4234-9234-123456789ABC",
            "mobux-12345678123442349234123456789abc",
            "mobux-{12345678-1234-4234-9234-123456789abc}",
            "12345678-1234-4234-9234-123456789abc",
            "=mobux-12345678-1234-4234-9234-123456789abc",
            "mobux-12345678-1234-4234-9234-123456789abc:0",
            "mobux-12345678-1234-4234-9234-123456789abc;kill-server",
            "mobux-12345678-1234-4234-9234-123456789abc\n",
        ] {
            assert!(bad.parse::<SessionId>().is_err(), "accepted {bad:?}");
            assert!(serde_json::from_value::<SessionId>(serde_json::json!(bad)).is_err());
        }
        assert!(!supported_version("tmux 3.3a\n"));
        assert!(!supported_version("tmux 3.4a\n"));
        assert!(!supported_version("tmux 3.5a\n"));
        assert!(supported_version("tmux 3.6\n"));
        assert!(supported_version("tmux 3.8"));
        assert!(supported_version("tmux 4.0"));
        assert!(!supported_version("tmux 3.2a"));
        assert!(!supported_version("tmux unknown"));
    }

    #[test]
    fn records_require_generation_names_ready_ownership_and_the_owned_pane() {
        let id = "mobux-12345678-1234-4234-9234-123456789abc";
        let spec = LaunchSpec {
            title: "literal title".into(),
            bin: "/bin/true".into(),
            args: vec![],
            cwd: PathBuf::from("/project"),
        };
        let encoded = STANDARD.encode(serde_json::to_vec(&spec).unwrap());
        let record = format!("{id}\t{OWNER}\t1\t{encoded}\t%0\t%0\t1\t7");
        let parsed = parse_record(&record).unwrap();
        assert_eq!(parsed.id.as_str(), id);
        assert_eq!(parsed.spec, spec);
        assert_eq!(parsed.status, SessionStatus::Exited(Some(7)));
        for invalid in [
            record.replacen(id, "$0", 1),
            record.replacen(OWNER, "other", 1),
            record.replacen("\t1\t", "\t0\t", 1),
            record.replacen("%0\t%0", "%0\t%1", 1),
            record.replacen(&encoded, "invalid base64", 1),
        ] {
            assert!(parse_record(&invalid).is_none(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn cache_paths_must_be_absolute_and_independent_of_working_directory() {
        assert_eq!(
            cache_directory(Some("/cache".into()), Some("/home/user".into())),
            Some(PathBuf::from("/cache"))
        );
        for relative in ["", "cache", "./cache", "../cache"] {
            assert_eq!(
                cache_directory(Some(relative.into()), Some("/home/user".into())),
                Some(PathBuf::from("/home/user/.cache"))
            );
            assert_eq!(
                cache_directory(Some(relative.into()), Some("relative-home".into())),
                None
            );
        }
        assert_eq!(
            cache_directory(None, Some("/home/user".into())),
            Some(PathBuf::from("/home/user/.cache"))
        );
        assert_eq!(cache_directory(None, Some("".into())), None);
        assert_eq!(cache_directory(None, None), None);
    }

    #[test]
    fn exact_identity_ignores_title_not_argv_or_directory() {
        let first = LaunchSpec {
            title: "one".into(),
            bin: "app".into(),
            args: vec!["a b".into(), "'quoted'".into()],
            cwd: PathBuf::from("/project A"),
        };
        let mut other = first.clone();
        other.title = "two".into();
        assert!(same_identity(&first, &other));
        other.args = vec!["a".into(), "b 'quoted'".into()];
        assert!(!same_identity(&first, &other));
        other = first.clone();
        other.cwd = PathBuf::from("/project B");
        assert!(!same_identity(&first, &other));
        other = first.clone();
        other.bin = "other-app".into();
        assert!(!same_identity(&first, &other));
    }

    #[test]
    fn reuses_pid_canonical_directory_and_title_independently() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let spec = server.spec("exec sleep 60");
        let first = server.backend.ensure(&spec).unwrap();
        let pid = server.pid(&first.id);
        let other_backend = server.backend.clone();
        let mut renamed = spec.clone();
        renamed.title = "renamed".into();
        renamed.cwd = renamed.cwd.join(".");
        for _ in 0..3 {
            assert_eq!(other_backend.ensure(&renamed).unwrap().id, first.id);
            assert_eq!(server.pid(&first.id), pid);
        }
        assert_eq!(server.backend.list().unwrap().len(), 1);
        assert!(server.backend.restart(&first.id).is_err());
        assert_eq!(server.pid(&first.id), pid);
        let permissions = fs::metadata(server.backend.lock_path.as_ref().unwrap())
            .unwrap()
            .permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }

    #[test]
    fn control_client_reattach_preserves_pid_and_death_detaches_only_its_session() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let session = server
            .backend
            .ensure(&server.spec("read line; exit 9"))
            .unwrap();
        let other = server
            .backend
            .ensure(&server.spec("exec sleep 60"))
            .unwrap();
        let pid = server.pid(&session.id);
        let mut first_client = TestClient::attach(&server, &session.id);
        let mut other_client = TestClient::attach(&server, &other.id);
        let clients = |count| {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let value = server
                    .backend
                    .run(&["list-clients", "-F", "#{client_name}\t#{session_name}"])
                    .unwrap();
                if value.lines().count() == count {
                    return value;
                }
                assert!(Instant::now() < deadline, "wrong client count: {value:?}");
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        let attached = clients(2);
        let current = attached
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .find(|(_, id)| *id == session.id.as_str())
            .unwrap()
            .0;
        server
            .backend
            .run(&["detach-client", "-t", current])
            .unwrap();
        first_client.disconnected();
        assert_eq!(server.pid(&session.id), pid);
        let mut reattached = TestClient::attach(&server, &session.id);
        clients(2);
        assert_eq!(server.pid(&session.id), pid);
        server
            .backend
            .run(&["send-keys", "-t", session.id.as_str(), "Enter"])
            .unwrap();
        assert_eq!(
            server.exited(&session.id).status,
            SessionStatus::Exited(Some(9))
        );
        reattached.disconnected();
        let remaining = clients(1);
        assert!(remaining.trim_end().ends_with(other.id.as_str()));
        assert!(other_client.0.try_wait().unwrap().is_none());
    }

    #[test]
    fn project_directories_and_argv_are_distinct_without_an_eight_session_cap() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let spec = server.spec("exec sleep 60");
        let first = server.backend.ensure(&spec).unwrap();
        let mut second = spec.clone();
        second.cwd = server.directory.join("project B 'quoted'");
        fs::create_dir(&second.cwd).unwrap();
        assert_ne!(first.id, server.backend.ensure(&second).unwrap().id);
        for index in 0..8 {
            let mut variant = spec.clone();
            variant.args.push(format!("literal argument {index}"));
            server.backend.ensure(&variant).unwrap();
        }
        assert_eq!(server.backend.list().unwrap().len(), 10);
    }

    #[test]
    fn immediate_exit_is_retained_and_only_explicit_restart_runs_again() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let spec = server.spec("printf x >> runs; exit 7");
        let first = server.backend.ensure(&spec).unwrap();
        let first = server.exited(&first.id);
        assert_eq!(first.status, SessionStatus::Exited(Some(7)));
        let pid = server.pid(&first.id);
        let handed_off = std::cell::Cell::new(false);
        assert!(server
            .backend
            .attach_with(&first, || {
                handed_off.set(true);
                Ok(())
            })
            .is_err());
        assert!(
            !handed_off.get(),
            "an exited app must not clear or take the terminal"
        );
        assert_eq!(server.backend.ensure(&spec).unwrap(), first);
        assert_eq!(server.pid(&first.id), pid);
        assert_eq!(fs::read(server.directory.join("runs")).unwrap(), b"x");
        assert!(server
            .backend
            .attach(&first)
            .unwrap_err()
            .contains("exited"));
        server.backend.restart(&first.id).unwrap();
        server.exited(&first.id);
        assert_eq!(fs::read(server.directory.join("runs")).unwrap(), b"xx");
        server.backend.stop(&first.id).unwrap();
        assert!(server.backend.list().unwrap().is_empty());
    }

    #[test]
    fn handoff_callback_runs_only_after_preflight_and_can_abort_without_killing() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let session = server
            .backend
            .ensure(&server.spec("while :; do sleep 1; done"))
            .unwrap();
        let pid = server.pid(&session.id);
        let called = std::cell::Cell::new(false);
        let error = server
            .backend
            .attach_with(&session, || {
                called.set(true);
                Err("handoff failed".into())
            })
            .unwrap_err();
        assert_eq!(error, "handoff failed");
        assert!(called.get());
        assert_eq!(server.pid(&session.id), pid);
        assert_eq!(
            server.backend.list().unwrap()[0].status,
            SessionStatus::Running
        );
    }

    #[test]
    fn unrelated_and_incomplete_records_are_ignored_and_cannot_be_stopped() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let unrelated = SessionId(format!("mobux-{}", Uuid::new_v4()));
        server
            .backend
            .run(&["new-session", "-d", "-s", unrelated.as_str()])
            .unwrap();
        assert!(server.backend.list().unwrap().is_empty());
        assert!(server.backend.stop(&unrelated).is_err());
        assert!(server.backend.restart(&unrelated).is_err());
        server
            .backend
            .run(&[
                "set-option",
                "-t",
                unrelated.as_str(),
                "@mobux-owner",
                OWNER,
            ])
            .unwrap();
        assert!(server.backend.list().unwrap().is_empty());
        let own = server
            .backend
            .ensure(&server.spec("exec sleep 60"))
            .unwrap();
        server.backend.stop(&own.id).unwrap();
        assert!(server
            .backend
            .run(&["has-session", "-t", unrelated.as_str()])
            .is_ok());
    }

    #[test]
    fn stale_actions_cannot_reach_reused_numeric_ids_on_a_new_server() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let spec = server.spec("exec sleep 60");
        let old = server.backend.ensure(&spec).unwrap();
        let numeric_ids = |id: &SessionId| {
            server
                .backend
                .run(&[
                    "display-message",
                    "-p",
                    "-t",
                    &id.target(),
                    "#{session_id} #{pane_id}",
                ])
                .unwrap()
        };
        assert_eq!(numeric_ids(&old.id).trim(), "$0 %0");
        server.backend.stop(&old.id).unwrap();
        assert!(server.backend.list().unwrap().is_empty());
        let new = server.backend.ensure(&spec).unwrap();
        assert_eq!(numeric_ids(&new.id).trim(), "$0 %0");
        assert_ne!(new.id, old.id);
        let pid = server.pid(&new.id);
        assert!(server.backend.stop(&old.id).is_err());
        assert!(server.backend.restart(&old.id).is_err());
        assert!(server.backend.attach(&old).is_err());
        assert!(server.backend.owned(&old.id).is_err());
        assert!(server.backend.respawn(&old.id, &old.spec).is_err());
        assert_eq!(server.pid(&new.id), pid);
        assert_eq!(
            server.backend.owned(&new.id).unwrap().status,
            SessionStatus::Running
        );
        assert!(Command::new("kill")
            .args(["-0", pid.trim()])
            .status()
            .unwrap()
            .success());
    }

    #[test]
    fn live_uuid_collision_never_reuses_or_cleans_up_the_existing_session() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let existing = server
            .backend
            .ensure(&server.spec("exec sleep 60"))
            .unwrap();
        let pid = server.pid(&existing.id);
        assert!(server
            .backend
            .create_named(server.spec("exit 9"), existing.id.clone())
            .is_err());
        assert_eq!(server.backend.owned(&existing.id).unwrap(), existing);
        assert_eq!(server.pid(&existing.id), pid);
    }

    #[test]
    fn qualified_respawn_cannot_follow_a_pane_in_another_session() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let exited = server.backend.ensure(&server.spec("exit 7")).unwrap();
        server.exited(&exited.id);
        let running = server
            .backend
            .ensure(&server.spec("exec sleep 60"))
            .unwrap();
        let pid = server.pid(&running.id);
        let foreign_pane = server
            .backend
            .run(&[
                "show-options",
                "-qv",
                "-t",
                &running.id.target(),
                "@mobux-pane",
            ])
            .unwrap();
        server
            .backend
            .run(&[
                "set-option",
                "-t",
                &exited.id.target(),
                "@mobux-pane",
                foreign_pane.trim(),
            ])
            .unwrap();
        assert!(server.backend.respawn(&exited.id, &exited.spec).is_err());
        assert_eq!(server.pid(&running.id), pid);
        assert_eq!(
            server.backend.owned(&running.id).unwrap().status,
            SessionStatus::Running
        );
    }

    #[test]
    fn missing_restart_directory_preserves_process_and_spawn_guard_fails_closed() {
        let Some(server) = TestServer::new() else {
            return;
        };
        for rename in [false, true] {
            let label = if rename { "renamed" } else { "deleted" };
            let marker = server.directory.join(format!("{label}-runs"));
            let mut spec = server.spec("printf x >> \"$1\"; exit 7");
            spec.args
                .extend(["record-run".into(), marker.to_str().unwrap().into()]);
            spec.cwd = server.directory.join(label);
            fs::create_dir(&spec.cwd).unwrap();
            let session = server.backend.ensure(&spec).unwrap();
            let session = server.exited(&session.id);
            let pid = server.pid(&session.id);
            if rename {
                fs::rename(&spec.cwd, spec.cwd.with_extension("moved")).unwrap();
            } else {
                fs::remove_dir(&spec.cwd).unwrap();
            }
            assert!(server
                .backend
                .restart(&session.id)
                .unwrap_err()
                .contains("launch directory"));
            assert_eq!(server.pid(&session.id), pid);
            assert_eq!(server.backend.owned(&session.id).unwrap(), session);
            assert_eq!(fs::read(&marker).unwrap(), b"x");

            // Deterministically model disappearance AFTER canonical_spec: call
            // the actual spawn path with the previously canonical stored spec.
            server.backend.respawn(&session.id, &session.spec).unwrap();
            assert_eq!(
                server.exited(&session.id).status,
                SessionStatus::Exited(Some(125))
            );
            assert_eq!(fs::read(&marker).unwrap(), b"x");
        }
    }

    #[test]
    fn arguments_are_literal_and_zero_argument_binary_is_not_a_shell_string() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let mut spec = server.spec("printf '%s\\n' \"$@\" > arguments");
        spec.args.push("test-program".into());
        let literal = [
            "a b",
            "'quotes'",
            "$(touch INJECTED)",
            "; touch INJECTED",
            "\"double\"",
            "*",
            "",
            "line\nbreak",
            ";",
            "kill-server",
            "tail;",
            "\\;",
            "two\\\\;",
        ];
        spec.args
            .extend(literal.iter().map(|arg| (*arg).to_owned()));
        let session = server.backend.ensure(&spec).unwrap();
        server.exited(&session.id);
        assert_eq!(
            fs::read_to_string(server.directory.join("arguments")).unwrap(),
            format!("{}\n", literal.join("\n"))
        );
        assert!(!server.directory.join("INJECTED").exists());

        let executable = server.directory.join("literal ; touch INJECTED=1;");
        fs::copy("/usr/bin/true", &executable).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let no_args = LaunchSpec {
            title: "literal bin".into(),
            bin: executable.to_str().unwrap().into(),
            args: vec![],
            cwd: server.directory.clone(),
        };
        let session = server.backend.ensure(&no_args).unwrap();
        assert_eq!(
            server.exited(&session.id).status,
            SessionStatus::Exited(Some(0))
        );
        assert!(!server.directory.join("INJECTED=1").exists());
    }

    #[test]
    fn literal_directory_formats_are_not_expanded() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let mut spec = server.spec("pwd -P > actual-directory; exit 0");
        spec.cwd = server
            .directory
            .join("#(touch INJECTED) #{session_id} 'space';");
        fs::create_dir(&spec.cwd).unwrap();
        let session = server.backend.ensure(&spec).unwrap();
        assert_eq!(
            server.exited(&session.id).status,
            SessionStatus::Exited(Some(0))
        );
        let actual = fs::read_to_string(spec.cwd.join("actual-directory")).unwrap();
        assert_eq!(
            actual.trim(),
            fs::canonicalize(&spec.cwd).unwrap().to_str().unwrap()
        );
        assert!(!spec.cwd.join("INJECTED").exists());
    }

    #[test]
    fn creation_failure_cleans_only_the_new_session() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let unrelated = server
            .backend
            .run(&["new-session", "-d", "-P", "-F", "#{session_id}"])
            .unwrap();
        let mut spec = server.spec("exit 0");
        // Force metadata past the OS argv limit AFTER session allocation.
        spec.title = "x".repeat(3 * 1024 * 1024);
        assert!(server.backend.ensure(&spec).is_err());
        let sessions = server
            .backend
            .run(&["list-sessions", "-F", "#{session_id}"])
            .unwrap();
        assert_eq!(sessions, unrelated);
    }

    #[test]
    fn lock_contention_fails_immediately_across_instances() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let lock = server.backend.lock().unwrap();
        let start = Instant::now();
        let error = server
            .backend
            .clone()
            .ensure(&server.spec("exit 0"))
            .unwrap_err();
        assert!(error.contains("busy"));
        assert!(start.elapsed() < Duration::from_secs(1));
        drop(lock);
        server.backend.ensure(&server.spec("exit 0")).unwrap();
    }

    #[test]
    fn footer_and_hook_use_local_tmux_features() {
        let Some(server) = TestServer::new() else {
            return;
        };
        let session = server
            .backend
            .ensure(&server.spec("exec sleep 60"))
            .unwrap();
        for row in ["status-format[0]", "status-format[1]"] {
            assert_eq!(
                server
                    .backend
                    .run(&["show-options", "-v", "-t", session.id.as_str(), row])
                    .unwrap()
                    .trim(),
                FOOTER
            );
        }
        let keys = server
            .backend
            .run(&["list-keys", "-T", "root", "MouseUp1Status"])
            .unwrap();
        assert!(keys.contains("mouse_status_range"));
        assert!(keys.contains("@mobux-return apps"));
        assert!(!keys.contains("detach-client -a"));
        assert!(!keys.contains("run-shell"));
        let hooks = server
            .backend
            .run(&["show-hooks", "-t", session.id.as_str(), "pane-died"])
            .unwrap();
        assert!(hooks.contains("detach-client -s"));
        assert!(hooks.contains(session.id.as_str()));
        // Verify the nested padding arithmetic against tmux's actual formatter.
        let expanded = server
            .backend
            .run(&[
                "display-message",
                "-p",
                "-t",
                session.id.as_str(),
                &FOOTER.replace("#{client_width}", "41"),
            ])
            .unwrap();
        let visible = expanded
            .trim_end()
            .replace("#[range=user|home]", "")
            .replace("#[range=user|apps]", "")
            .replace("#[norange]", "");
        let visible = visible.trim_end();
        assert_eq!(visible.len(), 40, "expanded footer: {expanded:?}"); // trim_end removes the label's trailing space.
        assert!(visible.starts_with(" Home "));
        assert!(visible.ends_with(" Apps"));
    }
}
