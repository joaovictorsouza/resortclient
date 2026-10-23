//! Game log mirrored to a rotating file in `GAME_STORAGE_DIR` (Documents on iOS) plus an in-memory
//! tail that the status panel shows. Installed as the `LogPlugin::custom_layer`.

use bevy::log::BoxedLayer;
use bevy::log::tracing::field::{Field, Visit};
use bevy::log::tracing::{Event, Level, Subscriber};
use bevy::log::tracing_subscriber::Layer;
use bevy::log::tracing_subscriber::layer::Context;
use bevy::prelude::App;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// Name of the active log file; the previous one is kept as `game.log.1`.
pub const LOG_FILE_NAME: &str = "game.log";
/// The file is rotated once it grows past this size (so at most about twice this is kept).
pub const MAX_LOG_BYTES: u64 = 512 * 1024;
const TAIL_CAPACITY: usize = 40;

/// Fixed-size ring of the most recent log lines.
#[derive(Debug, Default)]
pub struct LogTail {
    lines: VecDeque<String>,
    capacity: usize,
}

impl LogTail {
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, line: String) {
        if self.capacity == 0 {
            return;
        }
        if self.lines.len() == self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// The last `n` lines, oldest first.
    pub fn last(&self, n: usize) -> Vec<String> {
        let skip = self.lines.len().saturating_sub(n);
        self.lines.iter().skip(skip).cloned().collect()
    }
}

/// Append-only log file that moves itself to `<name>.1` when it passes `max_bytes`.
pub struct RotatingFile {
    path: PathBuf,
    max_bytes: u64,
    file: File,
    size: u64,
}

impl RotatingFile {
    pub fn open(path: PathBuf, max_bytes: u64) -> io::Result<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let size = file.metadata()?.len();
        let mut log = Self {
            path,
            max_bytes,
            file,
            size,
        };
        // A leftover from a long previous session must not grow without bound.
        if log.size >= log.max_bytes {
            log.rotate()?;
        }
        Ok(log)
    }

    fn backup_path(&self) -> PathBuf {
        let mut name = self.path.clone().into_os_string();
        name.push(".1");
        PathBuf::from(name)
    }

    fn rotate(&mut self) -> io::Result<()> {
        fs::rename(&self.path, self.backup_path())?;
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        self.size = 0;
        Ok(())
    }

    /// Writes one line (a newline is added), rotating first when the file is full.
    pub fn write_line(&mut self, line: &str) -> io::Result<()> {
        if self.size >= self.max_bytes {
            self.rotate()?;
        }
        writeln!(self.file, "{line}")?;
        self.size += line.len() as u64 + 1;
        Ok(())
    }
}

/// `[  12.345s] INFO target: message`
pub fn format_line(elapsed_secs: f64, level: &str, target: &str, message: &str) -> String {
    format!("[{elapsed_secs:8.3}s] {level:<5} {target}: {message}")
}

/// Cuts a line to `max_chars` characters, marking the cut with an ellipsis.
pub fn truncate_line(line: &str, max_chars: usize) -> String {
    if line.chars().count() <= max_chars {
        return line.to_string();
    }
    let mut cut: String = line.chars().take(max_chars.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

struct Sinks {
    file: Option<RotatingFile>,
    tail: LogTail,
}

fn sinks() -> &'static Mutex<Sinks> {
    static SINKS: OnceLock<Mutex<Sinks>> = OnceLock::new();
    SINKS.get_or_init(|| {
        let file = std::env::var_os("GAME_STORAGE_DIR")
            .map(|dir| Path::new(&dir).join(LOG_FILE_NAME))
            .and_then(|path| RotatingFile::open(path, MAX_LOG_BYTES).ok());
        Mutex::new(Sinks {
            file,
            tail: LogTail::new(TAIL_CAPACITY),
        })
    })
}

fn start_instant() -> &'static Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now)
}

/// The last `n` log lines, oldest first (empty until logging started).
pub fn recent_lines(n: usize) -> Vec<String> {
    sinks().lock().map(|s| s.tail.last(n)).unwrap_or_default()
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else if !field.name().starts_with("log.") {
            // `log.*` fields are bookkeeping added when bridging the `log` crate.
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }
}

struct GameLogLayer;

impl<S: Subscriber> Layer<S> for GameLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // winit's iOS event loop warns on every frame; it would drown the tail in the panel.
        if event
            .metadata()
            .target()
            .starts_with("winit::platform_impl::ios")
        {
            return;
        }
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let level = match *event.metadata().level() {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };
        let message = format!("{}{}", visitor.message, visitor.fields);
        let line = format_line(
            start_instant().elapsed().as_secs_f64(),
            level,
            event.metadata().target(),
            &message,
        );
        if let Ok(mut sinks) = sinks().lock() {
            if let Some(file) = sinks.file.as_mut() {
                // A full disk must never take the game down.
                let _ = file.write_line(&line);
            }
            sinks.tail.push(line);
        }
    }
}

/// `LogPlugin::custom_layer`: mirrors every event that passes the log filter to the file and tail.
pub fn custom_layer(_app: &mut App) -> Option<BoxedLayer> {
    start_instant();
    Some(Box::new(GameLogLayer))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("game_log_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn tail_keeps_only_the_most_recent_lines() {
        let mut tail = LogTail::new(3);
        for i in 0..5 {
            tail.push(format!("line {i}"));
        }
        assert_eq!(tail.last(10), vec!["line 2", "line 3", "line 4"]);
        assert_eq!(tail.last(2), vec!["line 3", "line 4"]);
        assert!(tail.last(0).is_empty());
    }

    #[test]
    fn zero_capacity_tail_stores_nothing() {
        let mut tail = LogTail::new(0);
        tail.push("x".into());
        assert!(tail.last(5).is_empty());
    }

    #[test]
    fn line_format_has_time_level_target_and_message() {
        assert_eq!(
            format_line(1.5, "INFO", "game::net", "hello"),
            "[   1.500s] INFO  game::net: hello"
        );
    }

    #[test]
    fn truncate_marks_the_cut_and_counts_chars() {
        assert_eq!(truncate_line("short", 10), "short");
        assert_eq!(truncate_line("abcdefghij", 5), "abcd…");
        assert_eq!(truncate_line("ééééé", 3), "éé…");
    }

    #[test]
    fn file_rotates_to_backup_when_full() {
        let dir = temp_dir("rotate");
        let path = dir.join(LOG_FILE_NAME);
        let mut log = RotatingFile::open(path.clone(), 20).unwrap();
        log.write_line("0123456789").unwrap(); // 11 bytes
        log.write_line("abcdefghij").unwrap(); // 22 bytes total -> full
        log.write_line("third").unwrap(); // rotates first
        assert_eq!(fs::read_to_string(&path).unwrap(), "third\n");
        assert_eq!(
            fs::read_to_string(dir.join("game.log.1")).unwrap(),
            "0123456789\nabcdefghij\n"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn oversized_leftover_is_rotated_on_open() {
        let dir = temp_dir("leftover");
        let path = dir.join(LOG_FILE_NAME);
        fs::write(&path, "x".repeat(50)).unwrap();
        let mut log = RotatingFile::open(path.clone(), 20).unwrap();
        log.write_line("fresh").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "fresh\n");
        assert_eq!(
            fs::read_to_string(dir.join("game.log.1")).unwrap().len(),
            50
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn open_creates_missing_directories_and_appends() {
        let dir = temp_dir("append");
        let path = dir.join("nested").join(LOG_FILE_NAME);
        RotatingFile::open(path.clone(), 1000)
            .unwrap()
            .write_line("one")
            .unwrap();
        RotatingFile::open(path.clone(), 1000)
            .unwrap()
            .write_line("two")
            .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "one\ntwo\n");
        let _ = fs::remove_dir_all(dir);
    }
}
