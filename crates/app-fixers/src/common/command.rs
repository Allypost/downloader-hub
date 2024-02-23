use std::{
    ffi::OsStr,
    process::{Command, Output},
};

use thiserror::Error;

use crate::error::FixerError;

pub struct Cmd {
    inner: Command,
}
impl Cmd {
    pub fn new<T: AsRef<OsStr>>(program: T) -> Self {
        Self {
            inner: Command::new(program),
        }
    }

    pub fn arg<T: AsRef<OsStr>>(&mut self, arg: T) -> &mut Self {
        self.inner.arg(arg);
        self
    }

    pub fn args<I, T>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = T>,
        T: AsRef<OsStr>,
    {
        self.inner.args(args);
        self
    }

    pub fn output(&mut self) -> Result<CmdOutput, CmdError> {
        app_logger::debug!("Running command: {:?}", &self.inner);

        self.inner.output().map(Into::into).map_err(CmdError::Run)
    }

    pub fn success_output(&mut self) -> Result<CmdOutput, CmdError> {
        let output = self.output()?;

        if output.is_success() {
            Ok(output)
        } else {
            Err(CmdError::Failed(format!("{:?}", self), output))
        }
    }
}

impl std::fmt::Debug for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

pub struct CmdOutput {
    inner: Output,
}
impl CmdOutput {
    pub fn is_success(&self) -> bool {
        self.inner.status.success()
    }

    pub fn status(&self) -> i32 {
        self.inner.status.code().unwrap_or(-1)
    }

    pub fn stdout_raw(&self) -> &[u8] {
        &self.inner.stdout
    }

    pub fn stdout(&self) -> Result<String, CmdOutputErr> {
        Self::decode_string(self.inner.stdout.clone())
    }

    pub fn stderr_raw(&self) -> &[u8] {
        &self.inner.stderr
    }

    pub fn stderr(&self) -> Result<String, CmdOutputErr> {
        Self::decode_string(self.inner.stderr.clone())
    }

    fn decode_string(bytes: Vec<u8>) -> Result<String, CmdOutputErr> {
        let string = String::from_utf8(bytes)?;

        Ok(string)
    }
}

impl From<Output> for CmdOutput {
    fn from(output: Output) -> Self {
        Self { inner: output }
    }
}

impl std::fmt::Debug for CmdOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

#[derive(Debug, Error)]
pub enum CmdError {
    #[error(transparent)]
    Run(std::io::Error),
    #[error(
        "Command {0} failed with status {status:?} and output {output:?}",
        status = .1.status(),
        output = .1.stderr(),
    )]
    Failed(String, CmdOutput),
    #[error(transparent)]
    Decode(#[from] CmdOutputErr),
}

#[derive(Debug, Error)]
pub enum CmdOutputErr {
    #[error(transparent)]
    DecodeError(#[from] std::string::FromUtf8Error),
}

impl From<CmdOutputErr> for FixerError {
    fn from(err: CmdOutputErr) -> Self {
        Self::CommandError(err.into())
    }
}
