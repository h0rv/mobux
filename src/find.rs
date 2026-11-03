use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const TEXT_OUTPUT_LIMIT: usize = 512 * 1024;
const TEXT_TIMEOUT: Duration = Duration::from_secs(2);
const TEXT_POLL_INTERVAL: Duration = Duration::from_millis(10);

type TextResult = Result<Vec<Hit>, String>;

pub struct TextJob {
    cancel: mpsc::SyncSender<()>,
    result: mpsc::Receiver<TextResult>,
}

impl TextJob {
    pub fn spawn(cwd: PathBuf, query: String) -> Self {
        let (cancel, cancelled) = mpsc::sync_channel(1);
        let (finished, result) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let result = search_text_cancellable(&cwd, &query, &cancelled);
            let _ = finished.send(result);
        });
        Self { cancel, result }
    }

    pub fn poll(&mut self) -> Option<TextResult> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Text search worker stopped".into())),
        }
    }
}

impl Drop for TextJob {
    fn drop(&mut self) {
        // The worker owns cleanup, so replacing a job never waits on the UI thread.
        let _ = self.cancel.try_send(());
    }
}

#[derive(Default)]
struct TextOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exceeded: bool,
    read_error: Option<String>,
}

fn capture_text(mut pipe: impl Read, output: Arc<Mutex<TextOutput>>, stderr: bool) {
    let mut buffer = [0; 8192];
    loop {
        let count = match pipe.read(&mut buffer) {
            Ok(0) => return,
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => {
                output.lock().unwrap().read_error = Some(error.to_string());
                return;
            }
        };
        let mut output = output.lock().unwrap();
        let remaining = TEXT_OUTPUT_LIMIT - output.stdout.len() - output.stderr.len();
        let captured = count.min(remaining);
        let target = if stderr {
            &mut output.stderr
        } else {
            &mut output.stdout
        };
        target.extend_from_slice(&buffer[..captured]);
        if count > remaining {
            output.exceeded = true;
            return;
        }
    }
}

fn run_text_command(
    command: &mut Command,
    cancelled: &mpsc::Receiver<()>,
    timeout: Duration,
) -> Result<TextOutput, String> {
    if !matches!(cancelled.try_recv(), Err(mpsc::TryRecvError::Empty)) {
        return Err("text search cancelled".to_string());
    }
    let started = Instant::now();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "ripgrep is not installed".to_string())?;
    let output = Arc::new(Mutex::new(TextOutput::default()));
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stdout_output = Arc::clone(&output);
    let stderr_output = Arc::clone(&output);
    let stdout_reader = thread::spawn(move || capture_text(stdout, stdout_output, false));
    let stderr_reader = thread::spawn(move || capture_text(stderr, stderr_output, true));
    let status = loop {
        if output.lock().unwrap().exceeded {
            break Err("ripgrep output exceeded 512 KiB".to_string());
        }
        if started.elapsed() >= timeout {
            break Err("ripgrep timed out".to_string());
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(error) => break Err(error.to_string()),
        }
        let delay = TEXT_POLL_INTERVAL.min(timeout.saturating_sub(started.elapsed()));
        match cancelled.recv_timeout(delay) {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            _ => break Err("text search cancelled".to_string()),
        }
    };
    if status.is_err() {
        let _ = child.kill();
    }
    // Always reap, including cancellation and limit/error paths, before finishing.
    let reaped = child.wait();
    let stdout_joined = stdout_reader.join();
    let stderr_joined = stderr_reader.join();
    let mut output = output.lock().unwrap();
    let status = status?;
    reaped.map_err(|error| error.to_string())?;
    if stdout_joined.is_err() || stderr_joined.is_err() {
        return Err("ripgrep output reader failed".to_string());
    }
    if output.exceeded {
        return Err("ripgrep output exceeded 512 KiB".to_string());
    }
    if let Some(error) = output.read_error.take() {
        return Err(error);
    }
    if status.code() == Some(1) {
        output.stdout.clear();
    } else if !status.success() && output.stdout.is_empty() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(if error.trim().is_empty() {
            "ripgrep failed".to_string()
        } else {
            error.trim().to_string()
        });
    }
    Ok(std::mem::take(&mut *output))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindMode {
    Names,
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Indexed {
    pub rel: String,
    pub is_dir: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub title: String,
    pub detail: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub fn index_names(root: &Path) -> Vec<Indexed> {
    let mut out = Vec::new();
    walk(root, "", &mut out, 0);
    out
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<Indexed>, depth: usize) {
    if out.len() >= 500 || depth > 6 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut children = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        let is_dir = kind.is_dir()
            || (kind.is_symlink() && fs::metadata(&path).is_ok_and(|metadata| metadata.is_dir()));
        children.push((name, path, is_dir, kind.is_dir()));
    }
    children.sort_by_cached_key(|child| child.0.to_lowercase());
    for (name, path, is_dir, recurse) in children {
        if out.len() >= 500 {
            return;
        }
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        out.push(Indexed {
            rel: rel.clone(),
            is_dir,
        });
        // Index directory links as folders without walking their targets or cycles.
        if recurse {
            walk(&path, &rel, out, depth + 1);
        }
    }
}

pub fn filter_names(root: &Path, index: &[Indexed], query: &str) -> Vec<Hit> {
    let mut scored: Vec<(i32, &Indexed)> = index
        .iter()
        .filter_map(|item| score(query, &item.rel).map(|value| (value, item)))
        .collect();
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.rel.len().cmp(&b.1.rel.len()))
    });
    scored.truncate(40);
    scored
        .into_iter()
        .map(|(_, item)| Hit {
            title: item.rel.clone(),
            detail: if item.is_dir {
                "folder".to_string()
            } else {
                "file".to_string()
            },
            path: root.join(&item.rel),
            is_dir: item.is_dir,
        })
        .collect()
}

pub fn search_text(root: &Path, query: &str) -> Result<Vec<Hit>, String> {
    let (_cancel, cancelled) = mpsc::sync_channel(1);
    search_text_cancellable(root, query, &cancelled)
}

fn search_text_cancellable(root: &Path, query: &str, cancelled: &mpsc::Receiver<()>) -> TextResult {
    if query.trim().chars().count() < 2 {
        return Ok(Vec::new());
    }
    let mut command = Command::new("rg");
    command
        .args([
            "-n",
            "--color",
            "never",
            "--max-count",
            "1",
            "-S",
            "--glob",
            "!.git/**",
            "--glob",
            "!target/**",
            "--glob",
            "!node_modules/**",
            "-e",
            query,
        ])
        .current_dir(root);
    let output = run_text_command(&mut command, cancelled, TEXT_TIMEOUT)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut hits = Vec::new();
    for line in text.lines().take(30) {
        let Some(hit) = parse_rg(root, line) else {
            continue;
        };
        hits.push(hit);
    }
    Ok(hits)
}

fn parse_rg(root: &Path, line: &str) -> Option<Hit> {
    let (path, rest) = line.split_once(':')?;
    let (number, text) = rest.split_once(':')?;
    Some(Hit {
        title: path.to_string(),
        detail: format!("line {number}  {}", text.trim()),
        path: root.join(path),
        is_dir: false,
    })
}

fn score(query: &str, candidate: &str) -> Option<i32> {
    let query: Vec<char> = query.chars().flat_map(|ch| ch.to_lowercase()).collect();
    if query.is_empty() {
        return Some(0);
    }
    let chars: Vec<char> = candidate.chars().collect();
    let mut query_index = 0;
    let mut total = 0i32;
    let mut run = 0i32;
    for (index, ch) in chars.iter().enumerate() {
        let Some(lower) = ch.to_lowercase().next() else {
            continue;
        };
        if query_index < query.len() && lower == query[query_index] {
            total += 8;
            if run > 0 {
                total += 6;
            }
            if index == 0 || chars.get(index.wrapping_sub(1)) == Some(&'/') {
                total += 10;
            }
            run += 1;
            query_index += 1;
        } else {
            run = 0;
        }
    }
    if query_index == query.len() {
        Some(total)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn text_command_cancellation_stops_and_reaps_the_child() {
        let (cancel, cancelled) = mpsc::sync_channel(1);
        let (finished, result) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let result = run_text_command(
                Command::new("/bin/sleep").arg("10"),
                &cancelled,
                Duration::from_secs(5),
            );
            finished.send(result.map(|_| ())).unwrap();
        });
        thread::sleep(Duration::from_millis(30));
        cancel.send(()).unwrap();
        assert_eq!(
            result.recv_timeout(Duration::from_secs(1)).unwrap(),
            Err("text search cancelled".to_string())
        );
        worker.join().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn text_command_timeout_stops_and_reaps_the_child() {
        let (_cancel, cancelled) = mpsc::sync_channel(1);
        let started = Instant::now();
        let result = run_text_command(
            Command::new("/bin/sleep").arg("10"),
            &cancelled,
            Duration::from_millis(30),
        );
        assert_eq!(result.err().as_deref(), Some("ripgrep timed out"));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(unix)]
    #[test]
    fn text_command_bounds_continuous_output() {
        let (_cancel, cancelled) = mpsc::sync_channel(1);
        let result = run_text_command(
            Command::new("/usr/bin/yes").arg("bounded output"),
            &cancelled,
            TEXT_TIMEOUT,
        );
        assert_eq!(
            result.err().as_deref(),
            Some("ripgrep output exceeded 512 KiB")
        );
    }

    #[test]
    fn text_capture_shares_one_limit_between_stdout_and_stderr() {
        let output = Arc::new(Mutex::new(TextOutput::default()));
        capture_text(
            io::repeat(b'o').take((TEXT_OUTPUT_LIMIT / 2) as u64),
            Arc::clone(&output),
            false,
        );
        capture_text(
            io::repeat(b'e').take(TEXT_OUTPUT_LIMIT as u64),
            Arc::clone(&output),
            true,
        );
        let output = output.lock().unwrap();
        assert_eq!(output.stdout.len() + output.stderr.len(), TEXT_OUTPUT_LIMIT);
        assert!(output.exceeded);
    }

    #[test]
    fn fuzzy_name_match_prefers_a_path_that_contains_the_letters_in_order() {
        let root = Path::new("/tmp");
        let index = vec![
            Indexed {
                rel: "src/paint.rs".to_string(),
                is_dir: false,
            },
            Indexed {
                rel: "README.md".to_string(),
                is_dir: false,
            },
        ];
        let hits = filter_names(root, &index, "pai");
        assert_eq!(hits[0].title, "src/paint.rs");
    }

    #[test]
    fn text_search_treats_leading_dashes_as_patterns() {
        let dir = std::env::temp_dir().join(format!("mobux-find-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("flags.txt"), "-n and --files are text\n").unwrap();
        for query in ["-n", "--files"] {
            let hits = search_text(&dir, query).unwrap();
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].path, dir.join("flags.txt"));
            assert_eq!(hits[0].detail, "line 1  -n and --files are text");
            assert!(!hits[0].title.contains('\u{1b}'));
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn directory_links_are_searchable_without_traversing_targets_or_cycles() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!("mobux-index-links-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let folder = root.join("real folder");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("child.txt"), "contents").unwrap();
        symlink(&folder, root.join("linked folder")).unwrap();
        symlink(&root, folder.join("cycle")).unwrap();
        symlink(&root, root.join("self cycle")).unwrap();
        symlink(root.join("missing"), root.join("broken link")).unwrap();
        symlink(folder.join("child.txt"), root.join("linked file")).unwrap();

        let index = index_names(&root);
        let mut actual: Vec<_> = index
            .iter()
            .map(|item| (item.rel.as_str(), item.is_dir))
            .collect();
        actual.sort_unstable();
        assert_eq!(
            actual,
            vec![
                ("broken link", false),
                ("linked file", false),
                ("linked folder", true),
                ("real folder", true),
                ("real folder/child.txt", false),
                ("real folder/cycle", true),
                ("self cycle", true),
            ]
        );
        let hits = filter_names(&root, &index, "linked folder");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, root.join("linked folder"));
        assert!(hits[0].is_dir);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_query_lists_the_index() {
        let index = vec![Indexed {
            rel: "src".to_string(),
            is_dir: true,
        }];
        let hits = filter_names(Path::new("."), &index, "");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].is_dir);
    }
}
