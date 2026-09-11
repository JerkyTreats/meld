use crate::error::ApiError;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};

const OUTPUT_LIMIT: u64 = 4 * 1024 * 1024;

pub(super) fn io_error(error: std::io::Error) -> ApiError {
    ApiError::ProviderError(format!("Codex invocation: {error}"))
}

pub(super) fn invocation_directory() -> Result<tempfile::TempDir, ApiError> {
    let temp = std::env::temp_dir().canonicalize().map_err(io_error)?;
    let workspace = std::env::current_dir()
        .map_err(io_error)?
        .canonicalize()
        .map_err(io_error)?;
    if temp.starts_with(&workspace) {
        return Err(ApiError::ConfigError(
            "Codex temporary root must be outside the current workspace".into(),
        ));
    }
    tempfile::Builder::new()
        .prefix("meld-codex-")
        .tempdir_in(temp)
        .map_err(io_error)
}

struct Invocation {
    child: Child,
    #[cfg(unix)]
    group: rustix::process::Pid,
}

impl Drop for Invocation {
    fn drop(&mut self) {
        // Kill the exact process group created by this invocation, including
        // descendants that still hold output pipes on timeout or cancellation.
        #[cfg(unix)]
        let _ = rustix::process::kill_process_group(self.group, rustix::process::Signal::KILL);
        let _ = self.child.start_kill();
    }
}

pub(super) struct Output {
    pub stdout: String,
    pub stderr: String,
}

async fn read_bounded(reader: impl AsyncRead + Unpin) -> Result<String, ApiError> {
    let mut bytes = Vec::new();
    reader
        .take(OUTPUT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(io_error)?;
    if bytes.len() as u64 > OUTPUT_LIMIT {
        return Err(ApiError::ProviderError(
            "Codex output exceeded the 4 MiB capture limit".into(),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|error| ApiError::ProviderError(format!("Codex output is not UTF-8: {error}")))
}

pub(super) async fn run(
    binary: &Path,
    args: &[String],
    cwd: &Path,
    auth_home: &Path,
    input: &str,
    timeout: Duration,
) -> Result<Output, ApiError> {
    let home = cwd.join("home");
    std::fs::create_dir_all(&home).map_err(io_error)?;
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("CODEX_HOME", auth_home)
        .env("TMPDIR", cwd)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for name in [
        "CODEX_API_KEY",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "HTTPS_PROXY",
        "HTTP_PROXY",
        "NO_PROXY",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    #[cfg(unix)]
    command.process_group(0);
    let child = command.spawn().map_err(io_error)?;
    #[cfg(unix)]
    let group = rustix::process::Pid::from_raw(child.id().expect("spawned child has pid") as i32)
        .expect("positive pid");
    let mut invocation = Invocation {
        child,
        #[cfg(unix)]
        group,
    };
    let mut stdin = invocation.child.stdin.take().expect("piped stdin");
    let stdout = invocation.child.stdout.take().expect("piped stdout");
    let stderr = invocation.child.stderr.take().expect("piped stderr");
    let result = tokio::time::timeout(timeout, async {
        let write = async {
            let result = stdin.write_all(input.as_bytes()).await;
            drop(stdin);
            Ok::<_, ApiError>(result)
        };
        let (written, stdout, stderr, status) =
            tokio::try_join!(write, read_bounded(stdout), read_bounded(stderr), async {
                invocation.child.wait().await.map_err(io_error)
            })?;
        if !status.success() {
            return Err(ApiError::ProviderExecutionFailed {
                message: format!("Codex exited {status}: {}", stderr.trim()),
                metadata: std::collections::BTreeMap::from([
                    ("stdout".into(), serde_json::json!(stdout)),
                    ("stderr".into(), serde_json::json!(stderr)),
                    ("exit_code".into(), serde_json::json!(status.code())),
                ]),
            });
        }

        written.map_err(io_error)?;
        Ok(Output { stdout, stderr })
    })
    .await;
    result.map_err(|_| {
        ApiError::ProviderRequestFailed(format!(
            "Codex completion timed out after {} ms",
            timeout.as_millis()
        ))
    })?
}
