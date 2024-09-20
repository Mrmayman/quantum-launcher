use std::{
    fmt::Display,
    process::ExitStatus,
    sync::{
        mpsc::{SendError, Sender},
        Arc, Mutex,
    },
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout},
};

use crate::{error::IoError, file_utils, io_err, json_structs::json_version::VersionDetails};

pub async fn read_logs_wrapped(
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Arc<Mutex<Child>>,
    sender: Sender<LogLine>,
    instance_name: String,
) -> Result<ExitStatus, String> {
    read_logs(stdout, stderr, child, sender, &instance_name)
        .await
        .map_err(|err| err.to_string())
}

pub async fn read_logs(
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Arc<Mutex<Child>>,
    sender: Sender<LogLine>,
    instance_name: &str,
) -> Result<ExitStatus, ReadError> {
    let uses_xml = is_xml(instance_name)?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut xml_cache = String::new();

    loop {
        let status = {
            let mut child = child.lock().unwrap();
            child.try_wait()
        };
        if let Ok(Some(status)) = status {
            // Game has exited.
            return Ok(status);
        }

        tokio::select! {
            line = stdout_reader.next_line() => {
                if let Some(line) = line? {
                    if uses_xml {
                        if line.starts_with("  </log4j:Event>") {
                            xml_cache.push_str(&line);
                            let xml = xml_cache.replace("<log4j:", "<").replace("</log4j:", "</");
                            let start = xml.find("<Event");

                            let text = if let Some(start) = start {
                                if start > 0 {
                                    let other_text = &xml[..start];
                                    sender.send(LogLine::Message(other_text.to_owned()))?;
                                    &xml[start..]
                                } else {
                                    &xml
                                }
                            } else {
                                &xml
                            };

                            match serde_xml_rs::from_str(&text) {
                                Ok(log_event) => {
                                    sender.send(LogLine::Info(log_event))?;
                                    xml_cache.clear();
                                },
                                Err(err) => {
                                    println!("[error] Could not parse XML: {err}\n{text}\n")
                                }
                            }

                        } else {
                            xml_cache.push_str(&format!("{line}\n"));
                        }
                    } else {
                        sender.send(LogLine::Message(format!("{line}\n")))?;
                    }
                } // else EOF
            },
            line = stderr_reader.next_line() => {
                if let Some(line) = line? {
                    sender.send(LogLine::Error(format!("{line}\n")))?;
                }
            }
        }
    }
}

fn is_xml(instance_name: &str) -> Result<bool, ReadError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance_name);
    let json_path = instance_dir.join("details.json");
    let json = std::fs::read_to_string(&json_path).map_err(io_err!(json_path))?;
    let json: VersionDetails = serde_json::from_str(&json)?;

    Ok(json.logging.is_some())
}

pub enum LogLine {
    Info(LogEvent),
    Message(String),
    Error(String),
}

pub enum ReadError {
    Io(std::io::Error),
    IoError(IoError),
    Send(SendError<LogLine>),
    Xml(serde_xml_rs::Error),
    Json(serde_json::Error),
}

impl Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Io(err) => write!(f, "error reading instance log: (io) {err}"),
            ReadError::Send(err) => write!(f, "error reading instance log: (send) {err}"),
            ReadError::Xml(err) => write!(f, "error reading instance log: (xml) {err}"),
            ReadError::IoError(err) => write!(f, "error reading instance log: (ioerror) {err}"),
            ReadError::Json(err) => write!(f, "error reading instance log: (json) {err}"),
        }
    }
}

impl From<std::io::Error> for ReadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_xml_rs::Error> for ReadError {
    fn from(value: serde_xml_rs::Error) -> Self {
        Self::Xml(value)
    }
}

impl From<SendError<LogLine>> for ReadError {
    fn from(value: SendError<LogLine>) -> Self {
        Self::Send(value)
    }
}

impl From<IoError> for ReadError {
    fn from(value: IoError) -> Self {
        Self::IoError(value)
    }
}

impl From<serde_json::Error> for ReadError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEvent {
    #[serde(rename = "logger")]
    pub logger: String,

    #[serde(rename = "timestamp")]
    pub timestamp: String,

    #[serde(rename = "level")]
    pub level: String,

    #[serde(rename = "thread")]
    pub thread: String,

    #[serde(rename = "Message")]
    pub message: LogMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogMessage {
    #[serde(rename = "$value")]
    pub content: String,
}
