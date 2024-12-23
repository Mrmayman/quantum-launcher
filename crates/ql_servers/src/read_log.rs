use std::{
    process::ExitStatus,
    sync::{
        mpsc::{SendError, Sender},
        Arc, Mutex,
    },
};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout},
};

pub async fn read_logs_wrapped(
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Arc<Mutex<Child>>,
    sender: Sender<String>,
) -> Result<ExitStatus, String> {
    read_logs(stdout, stderr, child, sender)
        .await
        .map_err(|err| err.to_string())
}

async fn read_logs(
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Arc<Mutex<Child>>,
    sender: Sender<String>,
) -> Result<ExitStatus, ReadError> {
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    loop {
        {
            let mut child = child.lock().unwrap();
            if let Ok(Some(status)) = child.try_wait() {
                // Game has exited.
                return Ok(status);
            }
        }

        tokio::select! {
            line = stdout_reader.next_line() => {
                if let Some(mut line) = line? {
                    line.push('\n');
                    sender.send(line)?;
                }
            }
            line = stderr_reader.next_line() => {
                if let Some(mut line) = line? {
                    line.push('\n');
                    sender.send(line)?;
                }
            }
        }
    }
}

pub enum ReadError {
    Io(std::io::Error),
    Send(SendError<String>),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error reading server logs: ")?;
        match self {
            ReadError::Io(err) => write!(f, "(io) {err}"),
            ReadError::Send(err) => write!(f, "(send) {err}"),
        }
    }
}

impl From<std::io::Error> for ReadError {
    fn from(err: std::io::Error) -> Self {
        ReadError::Io(err)
    }
}

impl From<SendError<String>> for ReadError {
    fn from(err: SendError<String>) -> Self {
        ReadError::Send(err)
    }
}
