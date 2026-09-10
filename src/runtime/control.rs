//! Operational cancellation enters the existing supervisor at its tick boundary.
//! HTTP acknowledgement is receipt only; durable drain outcome belongs to the
//! supervisor's existing shutdown record, identified by instance and start time.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::supervisor::{
    RuntimeDesiredState, RuntimeHealthSnapshot, RuntimeHeartbeat, RuntimeInstance, RuntimeLease,
    RuntimeShutdownState, SupervisorStore,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StopRequest {
    pub product_root: PathBuf,
    pub instance_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ControlStatus {
    pub product_root: PathBuf,
    pub process_id: u32,
    pub supervisor_store_path: PathBuf,
    pub instance: RuntimeInstance,
    pub stop_received: bool,
    pub shutdown: Option<RuntimeShutdownState>,
    pub participants: Vec<ParticipantStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantStatus {
    pub desired: RuntimeDesiredState,
    pub lease: Option<RuntimeLease>,
    pub heartbeat: Option<RuntimeHeartbeat>,
    pub health: Option<RuntimeHealthSnapshot>,
}

/// Live supervisor read handles and its cancellation flag; never a lifecycle writer.
pub struct RuntimeControl {
    product_root: PathBuf,
    instance_id: String,
    store: SupervisorStore,
    cancelled: Arc<AtomicBool>,
}

impl RuntimeControl {
    pub fn new(
        product_root: PathBuf,
        instance_id: String,
        store: SupervisorStore,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            product_root,
            instance_id,
            store,
            cancelled,
        }
    }

    pub fn instances(&self) -> Result<Vec<RuntimeInstance>, String> {
        self.store
            .list_runtime_instances()
            .map_err(|e| e.to_string())
    }

    pub fn shutdown(&self, instance: Option<&str>) -> Result<Option<RuntimeShutdownState>, String> {
        let id = instance.unwrap_or(&self.instance_id);
        let instance = self
            .store
            .get_runtime_instance(id)
            .map_err(|e| e.to_string())?
            .ok_or("runtime instance not found")?;
        self.store
            .get_shutdown_state(&shutdown_id(&instance))
            .map_err(|e| e.to_string())
    }

    pub fn status(&self) -> Result<ControlStatus, String> {
        let instance = self
            .store
            .get_runtime_instance(&self.instance_id)
            .map_err(|e| e.to_string())?
            .ok_or("supervisor instance is unavailable")?;
        let shutdown = self
            .store
            .get_shutdown_state(&shutdown_id(&instance))
            .map_err(|e| e.to_string())?;
        let participants = self
            .store
            .list_desired_runtime_state()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|desired| {
                let lease = self
                    .store
                    .get_active_runtime_lease(&desired.runtime_id)
                    .map_err(|e| e.to_string())?;
                let heartbeat = self
                    .store
                    .get_runtime_heartbeat(&desired.runtime_id)
                    .map_err(|e| e.to_string())?;
                let health = self
                    .store
                    .get_health_snapshot(&desired.runtime_id)
                    .map_err(|e| e.to_string())?;
                Ok(ParticipantStatus {
                    desired,
                    lease,
                    heartbeat,
                    health,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(ControlStatus {
            product_root: self.product_root.clone(),
            process_id: std::process::id(),
            supervisor_store_path: self.store.path().to_path_buf(),
            instance,
            stop_received: self.cancelled.load(Ordering::SeqCst),
            shutdown,
            participants,
        })
    }

    pub fn request_stop(&self, request: &StopRequest) -> Result<ControlStatus, String> {
        if request.product_root != self.product_root || request.instance_id != self.instance_id {
            return Err(
                "stop addresses another product or instance; rediscover before retrying".into(),
            );
        }
        self.cancelled.store(true, Ordering::SeqCst);
        self.status()
    }
}

pub fn shutdown_id(instance: &RuntimeInstance) -> String {
    format!(
        "shutdown:{}:{}",
        instance.instance_id, instance.started_at_ms
    )
}
