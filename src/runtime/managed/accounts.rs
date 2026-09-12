//! Bounded reads of the canonical managed process account stream.

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Component, Path};

use serde::Serialize;

use crate::error::ApiError;

#[derive(Debug, Serialize)]
pub(super) struct AccountPage {
    pub instance_id: String,
    pub records: Vec<serde_json::Value>,
    pub next_after: u64,
    pub more: bool,
}

pub(super) fn read(
    product_root: &Path,
    instance: &str,
    after: u64,
    limit: usize,
) -> Result<AccountPage, ApiError> {
    let mut components = Path::new(instance).components();
    if !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
        || Path::new(instance)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(instance)
        || limit == 0
        || limit > 1000
    {
        return Err(super::error(
            "expected one instance name and limit between 1 and 1000",
        ));
    }
    let file = std::fs::File::open(
        product_root
            .join("process-logs")
            .join(format!("{instance}.jsonl")),
    )
    .map_err(super::error)?;
    let length = file.metadata().map_err(super::error)?.len();
    if after > length {
        return Err(super::error("account cursor is beyond the retained log"));
    }
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(after)).map_err(super::error)?;
    let mut next_after = after;
    let mut records = Vec::new();
    // The foreground stream can also contain startup and shutdown output.
    // Read only complete lines, with byte and record bounds, so a concurrent
    // partial write remains available to the next read at the same cursor.
    while records.len() < limit && next_after - after < 4 * 1024 * 1024 {
        let mut line = Vec::new();
        let count = (&mut reader)
            .take(1024 * 1024)
            .read_until(b'\n', &mut line)
            .map_err(super::error)?;
        if count == 0 || line.last() != Some(&b'\n') {
            if count == 1024 * 1024 {
                return Err(super::error("account line exceeds the bounded read size"));
            }
            break;
        }
        next_after += count as u64;
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) {
            if value.get("type").and_then(|v| v.as_str()) == Some("runtime_tick_account") {
                records.push(value);
            }
        }
    }
    let more =
        next_after < length && (records.len() == limit || next_after - after >= 4 * 1024 * 1024);
    Ok(AccountPage {
        instance_id: instance.into(),
        records,
        next_after,
        more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_preserves_partial_lines_and_rejects_invalid_cursors() {
        let temp = tempfile::tempdir().unwrap();
        let logs = temp.path().join("process-logs");
        std::fs::create_dir(&logs).unwrap();
        let path = logs.join("instance.jsonl");
        let first = "{\"type\":\"runtime_tick_account\",\"tick\":1}\n";
        let partial = "{\"type\":\"runtime_tick_account\",\"tick\":2}";
        std::fs::write(&path, format!("startup\n{first}{partial}")).unwrap();
        let page = read(temp.path(), "instance", 0, 1).unwrap();
        assert_eq!(page.records.len(), 1);
        assert!(page.more);
        let pending = read(temp.path(), "instance", page.next_after, 1).unwrap();
        assert!(pending.records.is_empty());
        assert_eq!(pending.next_after, page.next_after);
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .unwrap()
            .write_all(b"\n")
            .unwrap();
        assert_eq!(
            read(temp.path(), "instance", page.next_after, 1)
                .unwrap()
                .records[0]["tick"],
            2
        );
        assert!(read(temp.path(), "../instance", 0, 1).is_err());
        assert!(read(temp.path(), "instance", 10000, 1).is_err());
    }
}
