use std::path::Path;

use super::*;

mod events;
mod provider;

fn executable(root: &Path, body: &str) -> OwnerExecutableV1 {
    let path = root.join("owner.py");
    let source = format!("#!/usr/bin/python3\nimport json,sys\n{body}\n");
    std::fs::write(&path, source.as_bytes()).unwrap();
    OwnerExecutableV1 {
        path,
        content_hash: blake3::hash(source.as_bytes()).to_hex().to_string(),
    }
}

fn connect(root: &Path, selected: &OwnerExecutableV1) -> OwnerConnection {
    OwnerConnection::start(
        selected,
        &root.join("implementations"),
        OwnerConnectionLimitsV1 {
            request_timeout_ms: 2_000,
            max_message_bytes: 65_536,
        },
    )
    .unwrap()
}

#[test]
fn external_revision_changes_without_changing_the_host_and_old_bytes_remain_available() {
    let root = tempfile::tempdir().unwrap();
    let body = |revision: &str| {
        format!("for line in sys.stdin:\n request=json.loads(line)\n print(json.dumps({{'message':'return','request_id':request['request_id'],'result':{{'Ok':'{revision}'}}}}),flush=True)")
    };
    let first = executable(root.path(), &body("owner-revision-one"));
    let mut old = connect(root.path(), &first);
    let read = |connection: &mut OwnerConnection| {
        connection
            .call::<String>(OwnerCommandV1::Describe, &NoOwnerCallbacks)
            .unwrap()
    };
    assert_eq!(read(&mut old), "owner-revision-one");
    let second = executable(root.path(), &body("owner-revision-two"));
    assert_ne!(first.content_hash, second.content_hash);
    assert!(
        matches!(OwnerConnection::start(&first, &root.path().join("implementations"), OwnerConnectionLimitsV1 { request_timeout_ms: 2_000, max_message_bytes: 65_536 }), Err(error) if error.code == "owner_implementation_changed")
    );
    let mut new = connect(root.path(), &second);
    assert_eq!(read(&mut new), "owner-revision-two");
    assert_eq!(read(&mut old), "owner-revision-one");
    let historical = OwnerExecutableV1 {
        path: root
            .path()
            .join("implementations")
            .join(&first.content_hash),
        ..first
    };
    let mut recovered = connect(root.path(), &historical);
    assert_eq!(read(&mut recovered), "owner-revision-one");
}

#[test]
fn installation_cannot_acquire_an_event_callback_grant_from_the_child() {
    let root = tempfile::tempdir().unwrap();
    let selected = executable(
        root.path(),
        r#"request=json.loads(sys.stdin.readline())
print(json.dumps({'message':'callback','request_id':request['request_id'],'callback_id':1,'callback':{'operation':'replay','request':{'cursor':{'ledger_id':'00000000-0000-0000-0000-000000000001','after_seq':0},'limit':1}}}),flush=True)
returned=json.loads(sys.stdin.readline())
print(json.dumps({'message':'return','request_id':request['request_id'],'result':returned['result']}),flush=True)"#,
    );
    let mut connection = connect(root.path(), &selected);
    let result = connection.call::<serde_json::Value>(OwnerCommandV1::Describe, &NoOwnerCallbacks);
    assert_eq!(result.unwrap_err().code, "owner_callback_not_granted");
}

#[test]
fn mismatched_operation_reply_cannot_complete_or_reuse_the_connection() {
    let root = tempfile::tempdir().unwrap();
    let selected = executable(
        root.path(),
        r#"request=json.loads(sys.stdin.readline())
print(json.dumps({'message':'return','request_id':request['request_id']+1,'result':{'Ok':'done'}}),flush=True)"#,
    );
    let mut connection = connect(root.path(), &selected);
    assert_eq!(
        connection
            .call::<String>(OwnerCommandV1::Flush, &NoOwnerCallbacks)
            .unwrap_err()
            .code,
        "owner_protocol_mismatch"
    );
    assert_eq!(
        connection
            .call::<String>(OwnerCommandV1::Flush, &NoOwnerCallbacks)
            .unwrap_err()
            .code,
        "owner_connection_unavailable"
    );
}

#[test]
fn external_theory_routes_install_and_reopen_through_the_canonical_router() {
    use crate::theory::*;
    let root = tempfile::tempdir().unwrap();
    let selected = executable(
        root.path(),
        r#"import os
owner='specimen'
route={'owner_domain':owner,'component_kind':'policy','route_version':1}
description={'protocol_version':1,'owner_id':owner,'routes':[{'route':route,'accepted_component_schema':{'min':1,'max':1},'package_cardinality':'many','handler_contract_version':1}],'capabilities':[],'implementations':[],'observation_participant':None}
for line in sys.stdin:
 request=json.loads(line)
 command=request['command']
 op=command['operation']
 result=None
 if op=='describe': result=description
 elif op=='open_revision_store':
  store=command['state_root']
  os.makedirs(store,exist_ok=True)
 elif op=='close_revision_store': pass
 elif op=='validate_theory':
  assert json.loads(bytes(command['canonical_bytes']))['policy']=='external semantics'
 elif op=='install_theory':
  import hashlib
  raw=bytes(command['canonical_bytes'])
  identity=hashlib.sha256(raw).hexdigest()
  result={'registry':'specimen-policy','id':command['owner_component_id'],'content_hash':identity}
  with open(os.path.join(store,identity),'wb') as f: f.write(raw)
 elif op=='verify_theory': assert os.path.isfile(os.path.join(store,command['reference']['content_hash']))
 elif op=='validate_links': assert command['component']['route']==route
 else: raise RuntimeError(op)
 print(json.dumps({'message':'return','request_id':request['request_id'],'result':{'Ok':result}}),flush=True)"#,
    );
    let open = || {
        registration::RegisteredOwner::open(
            "specimen",
            &selected,
            &root.path().join("implementations"),
            &root.path().join("owner-state"),
            OwnerConnectionLimitsV1 {
                request_timeout_ms: 2_000,
                max_message_bytes: 65_536,
            },
        )
        .unwrap()
    };
    let owner = open();
    let manifest = PdsPackageManifestV1 {
        schema_version: 1,
        package_id: "external-specimen".into(),
        package_version: "1".into(),
        description: None,
        imports: vec![],
        components: vec![PdsComponentEntry {
            component_id: "policy".into(),
            owner_component_id: "specimen-policy".into(),
            route: TheoryRouteId::new("specimen", "policy", 1),
            component_schema_version: 1,
            content: ComponentContentRef::Embedded {
                canonical_bytes: br#"{"policy":"external semantics"}"#.to_vec(),
            },
            requires: vec![],
        }],
    };
    let package = manifest.materialize(root.path()).unwrap();
    let store = PdsPackageStore::new(sled::open(root.path().join("core-state")).unwrap()).unwrap();
    let receipt = {
        let router = TheoryRouter::new(
            TheoryRouteCatalog::build(owner.route_handlers()).unwrap(),
            store.clone(),
        );
        router.install(&package, 0, false, None).unwrap()
    };
    assert_eq!(receipt.components.len(), 1);
    drop(owner);
    let reopened = open();
    let resolver = PdsPackageResolver::new(
        TheoryRouteCatalog::build(reopened.route_handlers()).unwrap(),
        store,
    );
    assert_eq!(
        resolver.resolve(&receipt.receipt_id).unwrap().receipt,
        receipt
    );
}

#[test]
fn interrupted_reads_retain_the_partial_owner_product() {
    struct InterruptedFrame {
        step: usize,
    }
    impl std::io::Read for InterruptedFrame {
        fn read(&mut self, target: &mut [u8]) -> std::io::Result<usize> {
            self.step += 1;
            let part: &[u8] = match self.step {
                1 => b"{\"value\":",
                2 => return Err(std::io::ErrorKind::Interrupted.into()),
                3 => b"42}\n",
                _ => return Ok(0),
            };
            target[..part.len()].copy_from_slice(part);
            Ok(part.len())
        }
    }
    let mut reader = std::io::BufReader::new(InterruptedFrame { step: 0 });
    let value: serde_json::Value = super::server::read_message(&mut reader, 64)
        .unwrap()
        .unwrap();
    assert_eq!(value, serde_json::json!({"value":42}));
    assert!(
        super::server::read_message::<serde_json::Value>(&mut reader, 64)
            .unwrap()
            .is_none()
    );
}
