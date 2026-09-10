//! Materialize the exact bundled package for the canonical theory installer.

use std::fs;
use std::path::PathBuf;

use crate::config::xdg;
use crate::error::ApiError;

const FILES: &[(&str, &[u8])] = &[
    (
        "authority_policy.startup.json",
        include_bytes!("../../theory/startup/authority_policy.startup.json"),
    ),
    (
        "belief_family.startup_realization.json",
        include_bytes!("../../theory/startup/belief_family.startup_realization.json"),
    ),
    (
        "curation_rule.startup.json",
        include_bytes!("../../theory/startup/curation_rule.startup.json"),
    ),
    (
        "epistemic_rule.startup.json",
        include_bytes!("../../theory/startup/epistemic_rule.startup.json"),
    ),
    (
        "evidence_mapping.startup_realization.json",
        include_bytes!("../../theory/startup/evidence_mapping.startup_realization.json"),
    ),
    (
        "graph_owner_event_route.nonce.json",
        include_bytes!("../../theory/startup/graph_owner_event_route.nonce.json"),
    ),
    (
        "maintained_condition.startup.json",
        include_bytes!("../../theory/startup/maintained_condition.startup.json"),
    ),
    (
        "pds-package.json",
        include_bytes!("../../theory/startup/pds-package.json"),
    ),
    (
        "product_topology.json",
        include_bytes!("../../theory/startup/product_topology.json"),
    ),
    (
        "strategy_theory.startup.json",
        include_bytes!("../../theory/startup/strategy_theory.startup.json"),
    ),
];

pub(super) fn package_source() -> Result<PathBuf, ApiError> {
    let mut hash = blake3::Hasher::new();
    for (name, content) in FILES {
        hash.update(name.as_bytes());
        hash.update(content);
    }
    let packages = xdg::data_home()
        .ok_or_else(|| error("cannot resolve XDG data home"))?
        .join("meld/packages");
    fs::create_dir_all(&packages).map_err(error)?;
    let root = packages.join(format!("startup-{}", hash.finalize().to_hex()));
    if !root.exists() {
        let pending = tempfile::Builder::new()
            .prefix(".startup-")
            .tempdir_in(&packages)
            .map_err(error)?;
        for (name, content) in FILES {
            fs::write(pending.path().join(name), content).map_err(error)?;
        }
        if let Err(failure) = fs::rename(pending.path(), &root) {
            if !root.is_dir() {
                return Err(error(failure));
            }
        }
    }
    for (name, content) in FILES {
        if fs::read(root.join(name)).map_err(error)? != *content {
            return Err(error(format!("bundled package content differs at {}; preserve it and use a separate XDG data root", root.join(name).display())));
        }
    }
    Ok(root)
}

fn error(message: impl std::fmt::Display) -> ApiError {
    ApiError::ConfigError(format!("Startup package: {message}"))
}
