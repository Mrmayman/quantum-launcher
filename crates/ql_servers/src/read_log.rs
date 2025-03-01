use std::{
    process::ExitStatus,
    sync::{
        mpsc::{SendError, Sender},
        Arc, Mutex,
    },
};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout},
};

/// Reads logs from a child process (server) and sends them to a sender.
///
/// Unlike the `read_logs` function in `ql_instances`
/// this one does not deal with XML parsing.
///
/// # Errors
/// - If an IO error was present when reading
///   from `stdout` or `stderr`
/// - If the `Receiver<String>` on the other
///   end was dropped.
pub async fn read_logs(
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Arc<Mutex<Child>>,
    sender: Sender<String>,
    name: String,
) -> Result<(ExitStatus, String), ReadError> {
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    loop {
        {
            let mut child = child.lock().unwrap();
            if let Ok(Some(status)) = child.try_wait() {
                // Game has exited.
                return Ok((status, name));
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

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("error reading server logs: {0}")]
    Io(#[from] std::io::Error),
    #[error("error reading server logs: {0}")]
    Send(#[from] SendError<String>),
}
