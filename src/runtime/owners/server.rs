use std::io::{BufRead, Write};

use serde::{de::DeserializeOwned, Serialize};

use super::contracts::*;

use std::sync::{Arc, Mutex};

/// Implemented by the package executable, never selected by semantic name in core.
pub trait PackageOwner {
    fn handle(
        &mut self,
        command: OwnerCommandV1,
        callbacks: Arc<dyn OwnerCallbackPort>,
    ) -> OwnerResult;
}

/// Serve an executable through the transport bounds supplied by its parent.
pub fn serve_stdio(owner: &mut dyn PackageOwner) -> Result<(), OwnerDiagnosticV1> {
    let limit = std::env::var("MELD_OWNER_MAX_MESSAGE_BYTES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            OwnerDiagnosticV1::new(
                "owner_binding_invalid",
                "parent transport bound is absent or invalid",
            )
        })?;
    serve_owner(
        owner,
        std::io::BufReader::new(std::io::stdin()),
        std::io::stdout(),
        limit,
    )
}

/// The child has no independent scheduler. Retained native capabilities use this
/// connection only while the host has an outstanding command and callback grant.
pub fn serve_owner(
    owner: &mut dyn PackageOwner,
    reader: impl BufRead + Send + 'static,
    writer: impl Write + Send + 'static,
    max_message_bytes: usize,
) -> Result<(), OwnerDiagnosticV1> {
    if max_message_bytes == 0 {
        return Err(OwnerDiagnosticV1::new(
            "owner_binding_invalid",
            "message bound must be positive",
        ));
    }
    let callbacks = Arc::new(ChildCallbacks {
        transport: Mutex::new(ChildTransport {
            reader: Box::new(reader),
            writer: Box::new(writer),
            request: None,
            max_message_bytes,
            failed: false,
        }),
    });
    let mut last_request_id = 0;
    loop {
        let (request_id, command) = {
            let mut transport = callbacks.lock()?;
            let Some(message) = read_message(&mut *transport.reader, max_message_bytes)? else {
                return Ok(());
            };
            let HostMessageV1::Command {
                protocol_version: OWNER_PROTOCOL_VERSION,
                request_id,
                command,
            } = message
            else {
                return Err(OwnerDiagnosticV1::new(
                    "owner_protocol_mismatch",
                    "expected a native command with the supported protocol",
                ));
            };
            if request_id <= last_request_id {
                return Err(OwnerDiagnosticV1::new(
                    "owner_protocol_mismatch",
                    "transport request identity did not advance",
                ));
            }
            last_request_id = request_id;
            transport.request = Some((request_id, 1));
            (request_id, command)
        };
        let result = owner.handle(*command, callbacks.clone());
        let mut transport = callbacks.lock()?;
        transport.request = None;
        if transport.failed {
            return Err(OwnerDiagnosticV1::new(
                "owner_connection_unavailable",
                "a callback transport failure invalidated this command",
            ));
        }
        write_message(
            &mut *transport.writer,
            &OwnerMessageV1::Return { request_id, result },
            max_message_bytes,
        )?;
    }
}

struct ChildCallbacks {
    transport: Mutex<ChildTransport>,
}
struct ChildTransport {
    reader: Box<dyn BufRead + Send>,
    writer: Box<dyn Write + Send>,
    request: Option<(u64, u64)>,
    max_message_bytes: usize,
    failed: bool,
}

impl ChildCallbacks {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ChildTransport>, OwnerDiagnosticV1> {
        self.transport.lock().map_err(|_| {
            OwnerDiagnosticV1::new(
                "owner_connection_unavailable",
                "owner callback connection lock poisoned",
            )
        })
    }
}

impl OwnerCallbackPort for ChildCallbacks {
    fn call(&self, callback: OwnerCallbackV1) -> OwnerResult {
        let mut transport = self.lock()?;
        if transport.failed {
            return Err(OwnerDiagnosticV1::new(
                "owner_connection_unavailable",
                "owner callback connection failed",
            ));
        }
        let Some((request_id, callback_id)) = transport.request else {
            return Err(OwnerDiagnosticV1::new(
                "owner_callback_not_granted",
                "no native command is outstanding",
            ));
        };
        let next = callback_id.checked_add(1).ok_or_else(|| {
            OwnerDiagnosticV1::new("owner_connection_exhausted", "callback identity exhausted")
        })?;
        transport.request = Some((request_id, next));
        let limit = transport.max_message_bytes;
        let exchange = (|| {
            write_message(
                &mut *transport.writer,
                &OwnerMessageV1::Callback {
                    request_id,
                    callback_id,
                    callback: Box::new(callback),
                },
                limit,
            )?;
            match read_message(&mut *transport.reader, limit)? {
                Some(HostMessageV1::CallbackReturn {
                    request_id: returned_request,
                    callback_id: returned_callback,
                    result,
                }) if returned_request == request_id && returned_callback == callback_id => {
                    Ok(result)
                }
                _ => Err(OwnerDiagnosticV1::new(
                    "owner_protocol_mismatch",
                    "callback reply belongs to another operation",
                )),
            }
        })();
        match exchange {
            Ok(result) => result,
            Err(error) => {
                transport.failed = true;
                Err(error)
            }
        }
    }
}

pub(crate) fn read_message<T: DeserializeOwned>(
    reader: &mut dyn BufRead,
    limit: usize,
) -> Result<Option<T>, OwnerDiagnosticV1> {
    let mut bytes = Vec::new();
    loop {
        let buffer = match reader.fill_buf() {
            Ok(buffer) => buffer,
            // A native stop signal does not cancel the outstanding owner product.
            // Keep its partial frame and let the supervisor drain after it returns.
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(OwnerDiagnosticV1::new("owner_transport_unavailable", error)),
        };
        if buffer.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            return Err(OwnerDiagnosticV1::new(
                "owner_protocol_incomplete",
                "owner closed an incomplete message",
            ));
        }
        let length = buffer
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(buffer.len(), |position| position + 1);
        if bytes.len().saturating_add(length) > limit {
            return Err(OwnerDiagnosticV1::new(
                "owner_message_limit",
                "owner message exceeds the declared transport bound",
            ));
        }
        let complete = buffer[length - 1] == b'\n';
        bytes.extend_from_slice(&buffer[..length]);
        reader.consume(length);
        if complete {
            break;
        }
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| OwnerDiagnosticV1::new("owner_protocol_invalid", error))
}

pub(crate) fn write_message(
    writer: &mut dyn Write,
    message: &impl Serialize,
    limit: usize,
) -> Result<(), OwnerDiagnosticV1> {
    let mut bytes = serde_json::to_vec(message)
        .map_err(|error| OwnerDiagnosticV1::new("owner_product_encoding", error))?;
    bytes.push(b'\n');
    if bytes.len() > limit {
        return Err(OwnerDiagnosticV1::new(
            "owner_message_limit",
            "owner message exceeds the declared transport bound",
        ));
    }
    writer
        .write_all(&bytes)
        .and_then(|_| writer.flush())
        .map_err(|error| OwnerDiagnosticV1::new("owner_transport_unavailable", error))
}

/// Translate native owner operations without inventing evidence in the transport.
/// Readiness owns the active context used by later bounded observation steps.
pub fn dispatch_observation(
    actor: &mut dyn crate::runtime::lifecycle::NativeObservationOwner,
    active: &mut Option<crate::runtime::lifecycle::ParticipantLifecycleContextV1>,
    command: OwnerCommandV1,
) -> OwnerResult {
    use OwnerCommandV1::*;
    let failure = |error: crate::runtime::error::RuntimeAssemblyError| {
        OwnerDiagnosticV1::new("owner_lifecycle_failed", error)
    };
    match command {
        Snapshot => encode_owner_result(actor.native_snapshot().map_err(failure)?),
        ResolvesWake { wake_ref } => {
            encode_owner_result(actor.native_resolves_wake(&wake_ref).map_err(failure)?)
        }
        Readiness { context } => {
            let receipt = actor.native_readiness(&context).map_err(failure)?;
            *active = Some(context);
            encode_owner_result(receipt)
        }
        Observe { context, budget } => {
            if active.as_ref() != Some(&context) {
                return Err(OwnerDiagnosticV1::new(
                    "owner_incarnation_mismatch",
                    "observation does not name the ready native incarnation",
                ));
            }
            encode_owner_result(actor.tick(budget))
        }
        Wait { context, report } => {
            encode_owner_result(actor.native_wait(&context, &report).map_err(failure)?)
        }
        SafePoint { context } => {
            encode_owner_result(actor.native_safe_point(&context).map_err(failure)?)
        }
        Stop { context } => {
            let receipt = actor.native_stop(&context).map_err(failure)?;
            *active = None;
            encode_owner_result(receipt)
        }
        Release { context } => {
            let receipt = actor.native_release(&context).map_err(failure)?;
            *active = None;
            encode_owner_result(receipt)
        }
        _ => Err(OwnerDiagnosticV1::new(
            "owner_operation_unsupported",
            "operation is not an observation lifecycle request",
        )),
    }
}

/// Installation executes the same narrow semantic ports as native Theory routing.
/// The parent retains package receipts, validation-token ownership and selection.
pub fn dispatch_theory(
    handler: &crate::theory::PortBackedTheoryRouteHandler,
    command: OwnerCommandV1,
) -> OwnerResult {
    use crate::theory::TheoryRouteHandler;
    use OwnerCommandV1::*;
    let failure = |error: crate::theory::OwnerRouteDiagnostic| {
        OwnerDiagnosticV1::new(&error.code, error.message)
    };
    let expected = handler.contract().route;
    match command {
        ValidateTheory {
            route,
            owner_component_id,
            canonical_bytes,
        } if route == expected => encode_owner_result(
            handler
                .validate_body(&owner_component_id, &canonical_bytes)
                .map_err(failure)?,
        ),
        InstallTheory {
            route,
            owner_component_id,
            canonical_bytes,
            installed_at_seq,
        } if route == expected => encode_owner_result(
            handler
                .install_body(&owner_component_id, &canonical_bytes, installed_at_seq)
                .map_err(failure)?,
        ),
        VerifyTheory { route, reference } if route == expected => {
            encode_owner_result(handler.verify_revision(&reference).map_err(failure)?)
        }
        ValidateLinks { component, package } if component.route == expected => encode_owner_result(
            handler
                .validate_links(&component, &package)
                .map_err(failure)?,
        ),
        _ => Err(OwnerDiagnosticV1::new(
            "owner_route_unavailable",
            "command does not address this owner's exact route",
        )),
    }
}
