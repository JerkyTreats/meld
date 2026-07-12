//! Runs the reusable authority suite against direct and serde-loopback clients.

#![cfg(feature = "test-support")]

use meld_events::error::EventAuthorityError;
use meld_events::events::remote::conformance::{
    assert_event_authority_conformance, EventAuthorityConformanceHarness,
};
use meld_events::events::remote::{
    EventAuthorityContract, LocalEventAuthorityClient, SerdeLoopbackEventAuthorityClient,
};
use meld_events::events::store::EventStore;
use meld_events::{EventAuthority, EventAuthorityOpenOptions, LedgerIdentity};

#[derive(Clone, Copy)]
enum ClientKind {
    Local,
    SerdeLoopback,
}

struct Harness {
    _dir: tempfile::TempDir,
    db: sled::Db,
    kind: ClientKind,
    ledger_id: LedgerIdentity,
    authority: Option<EventAuthority>,
    client: Option<Box<dyn EventAuthorityContract>>,
}

impl Harness {
    fn new(kind: ClientKind) -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let authority = EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default())
            .expect("fixture authority must open");
        let ledger_id = authority.ledger_identity();
        let client = Some(make_client(kind, &authority));
        Self {
            _dir: dir,
            db,
            kind,
            ledger_id,
            authority: Some(authority),
            client,
        }
    }
}

impl EventAuthorityConformanceHarness for Harness {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.ledger_id
    }

    fn contract(&self) -> &dyn EventAuthorityContract {
        self.client.as_deref().expect("fixture client is open")
    }

    fn reopen(&mut self) -> Result<(), EventAuthorityError> {
        self.client.take();
        self.authority.take();
        let authority = EventAuthority::open(
            self.db.clone(),
            EventAuthorityOpenOptions {
                expected_ledger_id: Some(self.ledger_id),
            },
        )?;
        self.client = Some(make_client(self.kind, &authority));
        self.authority = Some(authority);
        Ok(())
    }

    fn set_retained_from(&mut self, retained_from: u64) -> Result<(), EventAuthorityError> {
        EventStore::shared(self.db.clone())?
            .set_retained_lower_boundary(retained_from)
            .map_err(Into::into)
    }
}

fn make_client(kind: ClientKind, authority: &EventAuthority) -> Box<dyn EventAuthorityContract> {
    let local = LocalEventAuthorityClient::new(authority);
    match kind {
        ClientKind::Local => Box::new(local),
        ClientKind::SerdeLoopback => Box::new(SerdeLoopbackEventAuthorityClient::new(local)),
    }
}

#[test]
fn local_client_satisfies_authority_conformance() {
    assert_event_authority_conformance(&mut Harness::new(ClientKind::Local));
}

#[test]
fn serde_loopback_client_satisfies_authority_conformance() {
    assert_event_authority_conformance(&mut Harness::new(ClientKind::SerdeLoopback));
}
