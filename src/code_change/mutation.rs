//! Directory handles keep source traversal and atomic file replacement inside the bound tree.

use super::contracts::{FileReplacement, MAX_BYTES};
use rustix::fs::{open, openat, renameat, unlinkat, AtFlags, Mode, OFlags};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Component, Path},
};

pub(super) struct Workspace {
    root: File,
}

impl Workspace {
    pub fn open(path: &Path) -> Result<Self, String> {
        Ok(Self {
            root: File::from(
                open(
                    path,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|e| e.to_string())?,
            ),
        })
    }

    fn parent(&self, path: &str) -> Result<(File, String), String> {
        let mut parts: Vec<_> = Path::new(path).components().collect();
        let Some(Component::Normal(name)) = parts.pop() else {
            return Err("invalid code change path".into());
        };
        let mut parent = self.root.try_clone().map_err(|e| e.to_string())?;
        for part in parts {
            let Component::Normal(name) = part else {
                return Err("code change cannot escape its workspace".into());
            };
            parent = File::from(
                openat(
                    &parent,
                    name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|e| e.to_string())?,
            );
        }
        Ok((parent, name.to_string_lossy().into_owned()))
    }

    fn read(parent: &File, name: &str) -> Result<(Vec<u8>, std::fs::Permissions), String> {
        let file = File::from(
            openat(
                parent,
                name,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|e| e.to_string())?,
        );
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("code change requires existing regular files".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > MAX_BYTES {
            return Err("prior code content exceeds its byte bound".into());
        }
        Ok((bytes, metadata.permissions()))
    }

    pub fn content_hash(&self, path: &str) -> Result<String, String> {
        let (parent, name) = self.parent(path)?;
        let (bytes, _) = Self::read(&parent, &name)?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }

    pub fn identity(&self) -> Result<(u64, u64), String> {
        use std::os::unix::fs::MetadataExt;
        let metadata = self.root.metadata().map_err(|e| e.to_string())?;
        Ok((metadata.dev(), metadata.ino()))
    }

    fn stage_name(file: &FileReplacement, operation: &str) -> Result<String, String> {
        Ok(format!(
            ".meld-code-{}",
            &super::contracts::hash(&(operation, &file.relative_path))?[..32]
        ))
    }

    pub fn require_unclaimed_stage(
        &self,
        file: &FileReplacement,
        operation: &str,
    ) -> Result<(), String> {
        let (parent, _) = self.parent(&file.relative_path)?;
        match rustix::fs::statat(
            &parent,
            Self::stage_name(file, operation)?.as_str(),
            AtFlags::SYMLINK_NOFOLLOW,
        ) {
            Err(rustix::io::Errno::NOENT) => Ok(()),
            Ok(_) => Err("code change staging path is already occupied before intent".into()),
            Err(error) => Err(error.to_string()),
        }
    }

    fn cleanup_stage(parent: &File, stage: &str, replacement: &str) -> Result<(), String> {
        match rustix::fs::statat(parent, stage, AtFlags::SYMLINK_NOFOLLOW) {
            Err(rustix::io::Errno::NOENT) => return Ok(()),
            Ok(_) => {}
            Err(error) => return Err(error.to_string()),
        }
        let (bytes, _) = Self::read(parent, stage)?;
        if !replacement.as_bytes().starts_with(&bytes) {
            return Err("retained code staging path contains conflicting content".into());
        }
        unlinkat(parent, stage, AtFlags::empty()).map_err(|e| e.to_string())?;
        parent.sync_all().map_err(|e| e.to_string())
    }

    pub fn replace(&self, file: &FileReplacement, operation: &str) -> Result<(), String> {
        let (parent, name) = self.parent(&file.relative_path)?;
        let (before, permissions) = Self::read(&parent, &name)?;
        let stage = Self::stage_name(file, operation)?;
        if before == file.replacement.as_bytes() {
            return Self::cleanup_stage(&parent, &stage, &file.replacement);
        }
        if blake3::hash(&before).to_hex().as_str() != file.expected_content_hash {
            return Err(format!(
                "code source changed before replacement: {}",
                file.relative_path
            ));
        }
        let fd = openat(
            &parent,
            stage.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        );
        let fd = match fd {
            Ok(fd) => fd,
            Err(rustix::io::Errno::EXIST) => {
                // Intent reserves a previously absent name; recovery checks its partial bytes.
                Self::cleanup_stage(&parent, stage.as_str(), &file.replacement)?;
                openat(
                    &parent,
                    stage.as_str(),
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )
                .map_err(|e| e.to_string())?
            }
            Err(error) => return Err(error.to_string()),
        };
        let mut staged = File::from(fd);
        let result = (|| {
            staged
                .write_all(file.replacement.as_bytes())
                .map_err(|e| e.to_string())?;
            staged
                .set_permissions(permissions)
                .map_err(|e| e.to_string())?;
            staged.sync_all().map_err(|e| e.to_string())?;
            if Self::read(&parent, &name)?.0 != before {
                return Err("code source changed while preparing replacement".into());
            }
            renameat(&parent, stage.as_str(), &parent, name.as_str()).map_err(|e| e.to_string())?;
            parent.sync_all().map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = unlinkat(&parent, stage.as_str(), AtFlags::empty());
        }
        result
    }
}
