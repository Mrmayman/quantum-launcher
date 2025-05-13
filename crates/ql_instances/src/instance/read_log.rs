use std::{
    fmt::Display,
    process::ExitStatus,
    sync::{
        mpsc::{SendError, Sender},
        Arc, Mutex,
    },
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout},
};

use ql_core::{err, json::VersionDetails, IoError, JsonError, JsonFileError};

/// Reads log output from the given instance
/// and sends it to the given sender.
///
/// This async function runs till the instance process exits,
/// then it returns the exit status.
///
/// This automatically deals with XML logs.
///
/// # Arguments
/// - `stdout`: The stdout of the instance process.
/// - `stderr`: The stderr of the instance process.
/// - `child`: The instance process.
/// - `sender`: The sender to send [`LogLine`]s to.
/// - `instance_name`: The name of the instance.
///
/// # Errors
/// If:
/// - `details.json` couldn't be read or parsed into JSON
///   (for checking if XML logs are used)
/// - the `Receiver<LogLine>` was dropped,
///   disconnecting the channel
/// - Tokio *somehow* fails to read the `stdout` or `stderr`
#[allow(clippy::missing_panics_doc)]
pub async fn read_logs(
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Arc<Mutex<Child>>,
    sender: Sender<LogLine>,
    instance_name: String,
) -> Result<(ExitStatus, String), ReadError> {
    let uses_xml = is_xml(&instance_name).await?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut xml_cache = String::new();

    loop {
        let status = {
            // If the child has failed to lock
            // (because the `Mutex` was poisoned)
            // then we know something else has panicked,
            // so might as well panic too.
            //
            // (this is a methaphor for real life lol WTF: )
            let mut child = child.lock().unwrap();
            child.try_wait()
        };
        if let Ok(Some(status)) = status {
            // Game has exited.
            return Ok((status, instance_name));
        }

        tokio::select! {
            line = stdout_reader.next_line() => {
                if let Some(line) = line? {
                    if uses_xml {
                        xml_parse(&sender, &mut xml_cache, &line)?;
                    } else {
                        sender.send(LogLine::Message(line))?;
                    }
                } // else EOF
            },
            line = stderr_reader.next_line() => {
                if let Some(line) = line? {
                    sender.send(LogLine::Error(line))?;
                }
            }
        }
    }
}

fn xml_parse(
    sender: &Sender<LogLine>,
    xml_cache: &mut String,
    line: &str,
) -> Result<(), ReadError> {
    if !line.starts_with("  </log4j:Event>") {
        xml_cache.push_str(line);
        return Ok(());
    }

    xml_cache.push_str(line);
    let xml = xml_cache.replace("<log4j:", "<").replace("</log4j:", "</");
    let start = xml.find("<Event");

    let text = match start {
        Some(start) if start > 0 => {
            let other_text = xml[..start].trim();
            if !other_text.is_empty() {
                sender.send(LogLine::Message(other_text.to_owned()))?;
            }
            &xml[start..]
        }
        _ => &xml,
    };

    if let Ok(log_event) = serde_xml_rs::from_str(text) {
        sender.send(LogLine::Info(log_event))?;
        xml_cache.clear();
    } else {
        let no_unicode = any_ascii::any_ascii(text);
        match serde_xml_rs::from_str(&no_unicode) {
            Ok(log_event) => {
                sender.send(LogLine::Info(log_event))?;
                xml_cache.clear();
            }
            Err(err) => {
                err!("Could not parse XML: {err}\n{text}\n");
            }
        }
    }

    Ok(())
}

async fn is_xml(instance_name: &str) -> Result<bool, ReadError> {
    let json = VersionDetails::load(&ql_core::InstanceSelection::Instance(
        instance_name.to_owned(),
    ))
    .await?;

    Ok(json.logging.is_some())
}

/// Represents a line of log output.
///
/// # Variants
/// - `Info(LogEvent)`: A log event. Contains advanced
///   information about the log line like the timestamp,
///   class name, level and thread.
/// - `Message(String)`: A normal log message. Primarily
///   used for non-XML logs (old Minecraft versions).
/// - `Error(String)`: An error log message.
pub enum LogLine {
    Info(LogEvent),
    Message(String),
    Error(String),
}

impl Display for LogLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLine::Info(event) => write!(f, "{event}"),
            LogLine::Error(error) => write!(f, "! {error}"),
            LogLine::Message(message) => write!(f, "{message}"),
        }
    }
}

const READ_ERR_PREFIX: &str = "while reading the game log:\n";

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("{READ_ERR_PREFIX}{0}")]
    Io(#[from] std::io::Error),
    #[error("{READ_ERR_PREFIX}{0}")]
    IoError(#[from] IoError),
    #[error("{READ_ERR_PREFIX}send error: {0}")]
    Send(#[from] SendError<LogLine>),
    #[error("{READ_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
}

impl From<JsonFileError> for ReadError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => err.into(),
            JsonFileError::Io(err) => err.into(),
        }
    }
}

/// Represents a log event.
/// Contains advanced information about the log line
/// like the timestamp, class name, level and thread.
/// This is used for XML logs.
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEvent {
    pub logger: String,
    pub timestamp: String,
    pub level: String,
    pub thread: String,
    #[serde(rename = "Message")]
    pub message: LogMessage,
}

impl LogEvent {
    /// Returns the time of the log event, formatted as `HH:MM:SS`.
    #[must_use]
    pub fn get_time(&self) -> Option<String> {
        let time: i64 = self.timestamp.parse().ok()?;
        let seconds = time / 1000;
        let milliseconds = time % 1000;
        let nanoseconds = milliseconds * 1_000_000;
        let datetime = chrono::DateTime::from_timestamp(seconds, nanoseconds as u32)?;
        let datetime = datetime.with_timezone(&chrono::Local);
        Some(datetime.format("%H:%M:%S").to_string())
    }
}

impl Display for LogEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let date = self.get_time().unwrap_or_else(|| self.timestamp.clone());
        writeln!(
            f,
            "[{date}:{}:{}] [{}] {}",
            self.thread,
            self.logger,
            self.level,
            if let Some(n) = &self.message.content {
                &n
            } else {
                ""
            }
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogMessage {
    #[serde(rename = "$value")]
    pub content: Option<String>,
}
