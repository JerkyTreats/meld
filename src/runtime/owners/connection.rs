use std::io::{BufReader, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;

use super::contracts::*;
use super::server::{read_message, write_message};

/// One serial native owner connection. Every callback is handled through the
/// caller's operation-specific grant; no callback registry is selected by the child.
pub struct OwnerConnection {
    child: Child,
    process_group: Option<rustix::process::Pid>,
    stream: BufReader<UnixStream>,
    limits: OwnerConnectionLimitsV1,
    next_request_id: u64,
    failed: bool,
    executable_hash: String,
    retained_path: PathBuf,
}

impl OwnerConnection {
    /// Retain the exact selected executable so predecessor recovery does not follow
    /// an operator path whose bytes were replaced by a newer implementation.
    pub fn start(
        selected: &OwnerExecutableV1,
        implementation_root: &Path,
        limits: OwnerConnectionLimitsV1,
    ) -> Result<Self, OwnerDiagnosticV1> {
        if !selected.path.is_absolute()
            || !implementation_root.is_absolute()
            || limits.request_timeout_ms == 0
            || limits.max_message_bytes == 0
        {
            return Err(OwnerDiagnosticV1::new(
                "owner_binding_invalid",
                "executable and retention roots must be absolute and transport bounds positive",
            ));
        }
        let bytes = std::fs::read(&selected.path).map_err(io_error)?;
        let digest = blake3::hash(&bytes).to_hex().to_string();
        if digest != selected.content_hash {
            return Err(OwnerDiagnosticV1::new(
                "owner_implementation_changed",
                "selected executable differs from its exact digest",
            ));
        }
        std::fs::create_dir_all(implementation_root).map_err(io_error)?;
        let retained = implementation_root.join(&digest);
        if retained.exists() {
            if std::fs::read(&retained).map_err(io_error)? != bytes {
                return Err(OwnerDiagnosticV1::new(
                    "owner_implementation_corrupt",
                    "retained executable differs from its digest",
                ));
            }
        } else {
            let mut staged =
                tempfile::NamedTempFile::new_in(implementation_root).map_err(io_error)?;
            staged.write_all(&bytes).map_err(io_error)?;
            staged
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o700))
                .map_err(io_error)?;
            staged.as_file().sync_all().map_err(io_error)?;
            if let Err(error) = staged.persist_noclobber(&retained) {
                if std::fs::read(&retained).map_err(io_error)? != bytes {
                    return Err(io_error(error));
                }
            }
        }
        let (parent, child_socket) = UnixStream::pair().map_err(io_error)?;
        let input = Stdio::from(OwnedFd::from(child_socket.try_clone().map_err(io_error)?));
        let output = Stdio::from(OwnedFd::from(child_socket));
        let child = Command::new(&retained)
            .process_group(0)
            .env_clear()
            .env(
                "MELD_OWNER_MAX_MESSAGE_BYTES",
                limits.max_message_bytes.to_string(),
            )
            .current_dir(implementation_root)
            .stdin(input)
            .stdout(output)
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(io_error)?;
        Ok(Self {
            process_group: rustix::process::Pid::from_raw(child.id() as i32),
            child,
            stream: BufReader::new(parent),
            limits,
            next_request_id: 1,
            failed: false,
            executable_hash: digest,
            retained_path: retained,
        })
    }

    pub fn executable_hash(&self) -> &str {
        &self.executable_hash
    }

    pub fn call<T: DeserializeOwned>(
        &mut self,
        command: OwnerCommandV1,
        callbacks: &dyn OwnerCallbackPort,
    ) -> Result<T, OwnerDiagnosticV1> {
        if self.failed {
            return Err(OwnerDiagnosticV1::new(
                "owner_connection_unavailable",
                "the child connection failed; native recovery must reconnect it",
            ));
        }
        let result = self.exchange(command, callbacks);
        match result {
            Ok(result) => {
                let value = result?;
                serde_json::from_value(value)
                    .map_err(|error| OwnerDiagnosticV1::new("owner_product_invalid", error))
            }
            Err(error) => {
                self.failed = true;
                self.terminate_owned_processes();
                Err(error)
            }
        }
    }

    fn exchange(
        &mut self,
        command: OwnerCommandV1,
        callbacks: &dyn OwnerCallbackPort,
    ) -> Result<OwnerResult, OwnerDiagnosticV1> {
        let request_id = self.next_request_id;
        self.next_request_id = request_id.checked_add(1).ok_or_else(|| {
            OwnerDiagnosticV1::new(
                "owner_connection_exhausted",
                "transport request identity exhausted",
            )
        })?;
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(self.limits.request_timeout_ms))
            .ok_or_else(|| {
                OwnerDiagnosticV1::new("owner_binding_invalid", "transport timeout is out of range")
            })?;
        self.set_deadline(deadline)?;
        write_message(
            self.stream.get_mut(),
            &HostMessageV1::Command {
                protocol_version: OWNER_PROTOCOL_VERSION,
                request_id,
                command: Box::new(command),
            },
            self.limits.max_message_bytes,
        )?;
        let mut expected_callback_id = 1;
        loop {
            self.set_deadline(deadline)?;
            let message: OwnerMessageV1 =
                read_message(&mut self.stream, self.limits.max_message_bytes)?.ok_or_else(
                    || {
                        OwnerDiagnosticV1::new(
                            "owner_connection_closed",
                            "owner exited before returning the native operation",
                        )
                    },
                )?;
            match message {
                OwnerMessageV1::Return {
                    request_id: returned,
                    result,
                } if returned == request_id => return Ok(result),
                OwnerMessageV1::Callback {
                    request_id: returned,
                    callback_id,
                    callback,
                } if returned == request_id && callback_id == expected_callback_id => {
                    expected_callback_id =
                        expected_callback_id.checked_add(1).ok_or_else(|| {
                            OwnerDiagnosticV1::new(
                                "owner_connection_exhausted",
                                "callback identity exhausted",
                            )
                        })?;
                    let result = callbacks.call(*callback);
                    self.set_deadline(deadline)?;
                    write_message(
                        self.stream.get_mut(),
                        &HostMessageV1::CallbackReturn {
                            request_id,
                            callback_id,
                            result,
                        },
                        self.limits.max_message_bytes,
                    )?;
                }
                _ => {
                    return Err(OwnerDiagnosticV1::new(
                        "owner_protocol_mismatch",
                        "owner response is not for the outstanding native operation",
                    ))
                }
            }
        }
    }

    pub(crate) fn unavailable(&self) -> bool {
        self.failed
    }

    pub(crate) fn reconnect(&mut self) -> Result<(), OwnerDiagnosticV1> {
        let selected = OwnerExecutableV1 {
            path: self.retained_path.clone(),
            content_hash: self.executable_hash.clone(),
        };
        *self = Self::start(
            &selected,
            self.retained_path.parent().unwrap(),
            self.limits.clone(),
        )?;
        Ok(())
    }

    pub(crate) fn invalidate(&mut self) {
        self.failed = true;
        self.terminate_owned_processes();
    }

    fn terminate_owned_processes(&mut self) {
        if let Some(pid) = self.process_group.take() {
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
            let _ = self.child.wait();
        }
    }

    fn set_deadline(&self, deadline: Instant) -> Result<(), OwnerDiagnosticV1> {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|duration| !duration.is_zero())
            .ok_or_else(|| {
                OwnerDiagnosticV1::new(
                    "owner_operation_timed_out",
                    "native owner operation exceeded its declared transport deadline",
                )
            })?;
        self.stream
            .get_ref()
            .set_read_timeout(Some(remaining))
            .map_err(io_error)?;
        self.stream
            .get_ref()
            .set_write_timeout(Some(remaining))
            .map_err(io_error)
    }
}

impl Drop for OwnerConnection {
    fn drop(&mut self) {
        // Operational transport cleanup is not a semantic stop or completion receipt.
        self.terminate_owned_processes();
    }
}

fn io_error(error: impl ToString) -> OwnerDiagnosticV1 {
    OwnerDiagnosticV1::new("owner_transport_unavailable", error)
}
