//! Exact-process exit observation for managed stop completion on Linux.

use rustix::event::{poll, PollFd, PollFlags, Timespec};
use rustix::fd::OwnedFd;
use rustix::process::{pidfd_open, Pid, PidfdFlags};

use crate::error::ApiError;

pub(super) struct ExitWitness(Option<OwnedFd>);

impl ExitWitness {
    pub(super) fn capture(pid: Option<u32>) -> Result<Self, ApiError> {
        let Some(pid) = pid else {
            return Ok(Self(None));
        };
        let pid = i32::try_from(pid)
            .ok()
            .and_then(Pid::from_raw)
            .ok_or_else(|| super::error("invalid verified runtime PID"))?;
        match pidfd_open(pid, PidfdFlags::empty()) {
            Ok(fd) => Ok(Self(Some(fd))),
            Err(rustix::io::Errno::SRCH) => Ok(Self(None)),
            Err(error) => Err(super::error(format!(
                "cannot observe runtime exit: {error}"
            ))),
        }
    }

    pub(super) fn ready(&self) -> Result<bool, ApiError> {
        let Some(fd) = &self.0 else {
            return Ok(true);
        };
        let mut fds = [PollFd::new(fd, PollFlags::IN)];
        poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }),
        )
        .map_err(super::error)?;
        Ok(fds[0].revents().contains(PollFlags::IN))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn completion_waits_for_the_captured_process_to_exit() {
        let mut child = Command::new("sh")
            .args(["-c", "read marker"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let witness = ExitWitness::capture(Some(child.id())).unwrap();
        assert!(!witness.ready().unwrap());
        drop(child.stdin.take());
        child.wait().unwrap();
        assert!(witness.ready().unwrap());
    }
}
