use std::{
    fmt::Display,
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

pub async fn read_logs(
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Arc<Mutex<Child>>,
    sender: Sender<String>,
) -> Result<ExitStatus, ReadError> {
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // println!("start special read");
    // for line in stderr_reader {
    //     println!("{}", line?);
    // }
    // println!("stop special read");

    loop {
        let status = {
            let mut child = child.lock().unwrap();
            child.try_wait()
        };
        if let Ok(Some(status)) = status {
            // Game has exited.
            return Ok(status);
        }

        println!("starting read");
        // if let Some(line) = stdout_reader.next() {
        //     sender.send(line?)?;
        // }
        println!("finished stdout, starting stderr");
        // if let Some(line) = stderr_reader.next() {
        //     println!("sending stderr");
        //     sender.send(line?)?;
        // }
        println!("stopping read");
    }
}

pub enum ReadError {
    Io(std::io::Error),
    Send(SendError<String>),
}

impl Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Io(err) => write!(f, "error reading instance log: {err}"),
            ReadError::Send(err) => write!(f, "error reading instance log: {err}"),
        }
    }
}

impl From<std::io::Error> for ReadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<SendError<String>> for ReadError {
    fn from(value: SendError<String>) -> Self {
        Self::Send(value)
    }
}
