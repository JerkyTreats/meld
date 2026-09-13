//! Revision-local native graphs over shared immutable content and evidence.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, Weak};

use agdb::{
    CountComparison, Db, DbElement, DbErrorType, DbId, DbTransactionMut, QueryBuilder as Q,
    SyncMode,
};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::contracts::*;
use crate::error::StorageError;
use crate::events::DomainObjectRef;

type SharedDatabase = Arc<RwLock<Db>>;
static DATABASES: OnceLock<Mutex<BTreeMap<PathBuf, Weak<RwLock<Db>>>>> = OnceLock::new();

#[derive(Clone)]
pub(in crate::world_state::graph) struct PublicationStore {
    db: SharedDatabase,
    pub(in crate::world_state::graph) path: PathBuf,
}

/// Private revision descriptor. It is not an incomplete owner publication.
#[derive(Clone, Serialize, Deserialize)]
pub(in crate::world_state::graph) struct RevisionHeader {
    pub source_event: crate::events::EventRecordRef,
    pub source_route: Option<super::super::admission::OwnerEventSourceRef>,
    pub operation_id: String,
    pub event_record_id: String,
    pub enumeration_rule_revision: String,
    pub owner_id: String,
    pub revision_id: String,
    pub scope: OwnerPublicationScope,
    pub work_input_basis_id: Option<String>,
    pub completeness: OwnerCompletenessReceipt,
}

impl From<&ProjectedOwnerPublication> for RevisionHeader {
    fn from(publication: &ProjectedOwnerPublication) -> Self {
        let operation = &publication.operation;
        let batch = &operation.batch;
        Self {
            source_event: publication.source_event,
            source_route: publication.source_route.clone(),
            operation_id: operation.operation_id.clone(),
            event_record_id: operation.event_record_id(),
            enumeration_rule_revision: operation.enumeration_rule_revision.clone(),
            owner_id: batch.owner_id.clone(),
            revision_id: batch.revision_id.clone(),
            scope: batch.scope.clone(),
            work_input_basis_id: batch.work_input_basis_id.clone(),
            completeness: batch.completeness.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Member {
    id: String,
    content: i64,
    evidence: i64,
    ordinal: usize,
}

pub(in crate::world_state::graph) fn data(error: impl std::fmt::Display) -> StorageError {
    StorageError::InvalidPath(format!("Graph agdb: {error}"))
}

fn encode(value: &impl Serialize) -> Result<String, StorageError> {
    serde_json::to_string(value).map_err(data)
}

fn key(kind: &str, value: &impl Serialize) -> Result<String, StorageError> {
    Ok(format!(
        "{kind}:{}",
        blake3::hash(encode(value)?.as_bytes()).to_hex()
    ))
}

fn property<'a>(element: &'a DbElement, name: &str) -> Result<&'a str, StorageError> {
    element
        .values
        .iter()
        .find(|kv| kv.key == name.into())
        .ok_or_else(|| data(format!("missing {name} on {}", element.id.0)))?
        .value
        .string()
        .map(String::as_str)
        .map_err(data)
}

fn alias(db: &Db, name: &str) -> Result<Option<DbElement>, StorageError> {
    match db.exec(Q::select().ids(name).query()) {
        Ok(result) => Ok(result.elements.into_iter().next()),
        Err(error) if error.ty == DbErrorType::NotFound => Ok(None),
        Err(error) => Err(data(error)),
    }
}

fn header(element: &DbElement) -> Result<RevisionHeader, StorageError> {
    serde_json::from_str(property(element, "header")?).map_err(data)
}

fn event_alias(seq: u64) -> String {
    format!("event:{seq}")
}

fn address_alias(seq: u64, address: &DomainObjectRef) -> Result<String, StorageError> {
    key("address", &(seq, address))
}

impl PublicationStore {
    pub(in crate::world_state::graph) fn open(
        path: &Path,
        resource: &str,
    ) -> Result<Self, StorageError> {
        let parent = path
            .parent()
            .ok_or_else(|| data("Graph path requires a parent"))?;
        std::fs::create_dir_all(parent).map_err(StorageError::IoError)?;
        let path = parent.canonicalize().map_err(StorageError::IoError)?.join(
            path.file_name()
                .ok_or_else(|| data("Graph path requires a filename"))?,
        );
        let mut databases = DATABASES.get_or_init(Default::default).lock();
        databases.retain(|_, value| value.strong_count() > 0);
        let db = if let Some(db) = databases.get(&path).and_then(Weak::upgrade) {
            db
        } else {
            let mut db = Db::new(
                path.to_str()
                    .ok_or_else(|| data("Graph path must be UTF-8"))?,
            )
            .map_err(data)?;
            db.set_sync_mode(SyncMode::Commit);
            if let Some(bound) = alias(&db, "resource")? {
                if property(&bound, "identity")? != resource {
                    return Err(data("Graph file belongs to another world-model resource"));
                }
            } else {
                transact(&mut db, |tx| {
                    tx.exec_mut(
                        Q::insert()
                            .nodes()
                            .aliases("resource")
                            .values_uniform([("identity", resource).into()])
                            .query(),
                    )
                    .map_err(data)?;
                    for name in [
                        "selection",
                        "route-selection",
                        "revision-members",
                        "all-headers",
                    ] {
                        tx.exec_mut(Q::insert().index(name).query()).map_err(data)?;
                    }
                    Ok(())
                })?;
            }
            let db = Arc::new(RwLock::new(db));
            databases.insert(path.clone(), Arc::downgrade(&db));
            db
        };
        if property(
            &alias(&db.read(), "resource")?.ok_or_else(|| data("missing resource"))?,
            "identity",
        )? != resource
        {
            return Err(data("Graph file belongs to another world-model resource"));
        }
        Ok(Self { db, path })
    }

    pub(in crate::world_state::graph) fn flush(&self) -> Result<(), StorageError> {
        self.db.write().sync().map_err(data)
    }

    pub(in crate::world_state::graph) fn migration_complete(&self) -> Result<bool, StorageError> {
        Ok(alias(&self.db.read(), "sled-import-complete")?.is_some())
    }

    pub(in crate::world_state::graph) fn complete_migration(&self) -> Result<(), StorageError> {
        self.db
            .write()
            .exec_mut(Q::insert().nodes().aliases("sled-import-complete").query())
            .map_err(data)?;
        self.flush()
    }

    pub(in crate::world_state::graph) fn clear(&self) -> Result<(), StorageError> {
        let mut db = self.db.write();
        let keep = alias(&db, "resource")?
            .ok_or_else(|| data("missing resource"))?
            .id;
        let migration = alias(&db, "sled-import-complete")?.map(|e| e.id);
        let ids = db
            .exec(Q::search().elements().query())
            .map_err(data)?
            .elements
            .into_iter()
            .filter(|e| e.id != keep && Some(e.id) != migration)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        if !ids.is_empty() {
            db.exec_mut(Q::remove().ids(ids).query()).map_err(data)?;
        }
        Ok(())
    }

    #[tracing::instrument(target = "meld::trace", name = "graph.commit_publication", skip_all, fields(seq = publication.source_event.seq))]
    pub(in crate::world_state::graph) fn put(
        &self,
        publication: &ProjectedOwnerPublication,
    ) -> Result<(), StorageError> {
        publication.operation.validate()?;
        let mut db = self.db.write();
        let seq = publication.source_event.seq;
        if let Some(existing) = alias(&db, &event_alias(seq))? {
            if reconstruct(&db, &existing)? != *publication {
                return Err(data("Event position has divergent publication content"));
            }
            return Ok(());
        }
        let batch = &publication.operation.batch;
        let guard = key(
            "guard",
            &(&batch.owner_id, &batch.scope, &batch.revision_id),
        )?;
        if let Some(existing) = alias(&db, &guard)? {
            if property(&existing, "operation")? != publication.operation.operation_id {
                return Err(data("owner revision has divergent publication operations"));
            }
        }
        let selection = key("scope", &(&batch.owner_id, &batch.scope))?;
        let route_selection = key(
            "route",
            &(&batch.owner_id, &batch.scope, &publication.source_route),
        )?;
        let mut heads = Vec::new();
        for selection in [&selection, &route_selection] {
            let name = format!("head:{selection}");
            let prior = alias(&db, &name)?;
            let replace = prior
                .as_ref()
                .map(|e| property(e, "seq")?.parse::<u64>().map_err(data))
                .transpose()?
                .is_none_or(|old| old < seq);
            if replace {
                heads.push((name, prior.map(|e| e.id)));
            }
        }
        let metadata = RevisionHeader::from(publication);
        transact(&mut db, |tx| {
            let mut interned = BTreeMap::new();
            let mut addresses: BTreeMap<String, (&DomainObjectRef, Vec<Member>)> = BTreeMap::new();
            for (ordinal, object) in batch.objects.iter().enumerate() {
                let member = member(
                    tx,
                    &mut interned,
                    serde_json::to_value(object).map_err(data)?,
                    "publication_id",
                    ordinal,
                )?;
                addresses
                    .entry(encode(&object.object_ref)?)
                    .or_insert((&object.object_ref, Vec::new()))
                    .1
                    .push(member);
            }
            for edge in &batch.relations {
                addresses
                    .entry(encode(&edge.src)?)
                    .or_insert((&edge.src, Vec::new()));
                addresses
                    .entry(encode(&edge.dst)?)
                    .or_insert((&edge.dst, Vec::new()));
            }
            let mut ids = BTreeMap::new();
            for (address, (object_ref, members)) in addresses {
                let result = tx
                    .exec_mut(
                        Q::insert()
                            .nodes()
                            .aliases(address_alias(seq, object_ref)?.as_str())
                            .values_uniform(vec![
                                ("objects", encode(&members)?).into(),
                                ("revision-members", seq.to_string()).into(),
                            ])
                            .query(),
                    )
                    .map_err(data)?;
                ids.insert(address, result.elements[0].id);
            }
            for (ordinal, edge) in batch.relations.iter().enumerate() {
                let member = member(
                    tx,
                    &mut interned,
                    serde_json::to_value(edge).map_err(data)?,
                    "occurrence_id",
                    ordinal,
                )?;
                tx.exec_mut(
                    Q::insert()
                        .edges()
                        .from(ids[&encode(&edge.src)?])
                        .to(ids[&encode(&edge.dst)?])
                        .values_uniform(vec![
                            ("relation", encode(&member)?).into(),
                            ("revision-members", seq.to_string()).into(),
                        ])
                        .query(),
                )
                .map_err(data)?;
            }
            tx.exec_mut(
                Q::insert()
                    .nodes()
                    .aliases(event_alias(seq).as_str())
                    .values_uniform(vec![
                        ("header", encode(&metadata)?).into(),
                        ("selection", selection.clone()).into(),
                        ("route-selection", route_selection.clone()).into(),
                        ("all-headers", "yes").into(),
                    ])
                    .query(),
            )
            .map_err(data)?;
            tx.exec_mut(
                Q::insert()
                    .nodes()
                    .aliases(guard.as_str())
                    .values_uniform([
                        ("operation", publication.operation.operation_id.clone()).into()
                    ])
                    .query(),
            )
            .map_err(data)?;
            for (name, id) in &heads {
                let values = [("seq", seq.to_string()).into()];
                if let Some(id) = id {
                    tx.exec_mut(Q::insert().values_uniform(values).ids(*id).query())
                        .map_err(data)?;
                } else {
                    tx.exec_mut(
                        Q::insert()
                            .nodes()
                            .aliases(name.as_str())
                            .values_uniform(values)
                            .query(),
                    )
                    .map_err(data)?;
                }
            }
            Ok(())
        })
    }

    pub(in crate::world_state::graph) fn header_for_event(
        &self,
        seq: u64,
    ) -> Result<Option<RevisionHeader>, StorageError> {
        alias(&self.db.read(), &event_alias(seq))?
            .as_ref()
            .map(header)
            .transpose()
    }

    #[tracing::instrument(
        target = "meld::trace",
        name = "graph.select_revision",
        skip_all,
        fields(through)
    )]
    pub(in crate::world_state::graph) fn select(
        &self,
        requirement: &TraversalOwnerRequirement,
        through: u64,
    ) -> Result<Option<RevisionHeader>, StorageError> {
        let db = self.db.read();
        let (index, selection) = if let Some(source) = &requirement.event_source {
            (
                "route-selection",
                key(
                    "route",
                    &(&requirement.owner_id, &requirement.scope, &Some(source)),
                )?,
            )
        } else {
            (
                "selection",
                key("scope", &(&requirement.owner_id, &requirement.scope))?,
            )
        };
        if let Some(head) = alias(&db, &format!("head:{selection}"))? {
            let seq = property(&head, "seq")?.parse::<u64>().map_err(data)?;
            if seq <= through {
                return alias(&db, &event_alias(seq))?
                    .as_ref()
                    .map(header)
                    .transpose();
            }
        }
        let headers = db
            .exec(Q::select().search().index(index).value(selection).query())
            .map_err(data)?;
        let mut selected: Option<RevisionHeader> = None;
        for element in headers.elements {
            let candidate = header(&element)?;
            if candidate.source_event.seq <= through
                && selected
                    .as_ref()
                    .is_none_or(|old| old.source_event.seq < candidate.source_event.seq)
            {
                selected = Some(candidate);
            }
        }
        Ok(selected)
    }

    pub(in crate::world_state::graph) fn publication(
        &self,
        seq: u64,
    ) -> Result<Option<ProjectedOwnerPublication>, StorageError> {
        let db = self.db.read();
        alias(&db, &event_alias(seq))?
            .as_ref()
            .map(|e| reconstruct(&db, e))
            .transpose()
    }

    pub(in crate::world_state::graph) fn publications(
        &self,
        through: u64,
    ) -> Result<Vec<ProjectedOwnerPublication>, StorageError> {
        let db = self.db.read();
        let mut publications = Vec::new();
        for element in db
            .exec(
                Q::select()
                    .search()
                    .index("all-headers")
                    .value("yes")
                    .query(),
            )
            .map_err(data)?
            .elements
        {
            if header(&element)?.source_event.seq <= through {
                publications.push(reconstruct(&db, &element)?);
            }
        }
        publications.sort_by_key(|p| p.source_event.seq);
        Ok(publications)
    }

    pub(in crate::world_state::graph) fn objects(
        &self,
        seq: u64,
        address: &DomainObjectRef,
    ) -> Result<Vec<OwnerObjectPublication>, StorageError> {
        let db = self.db.read();
        let Some(element) = alias(&db, &address_alias(seq, address)?)? else {
            return Ok(Vec::new());
        };
        let members: Vec<Member> =
            serde_json::from_str(property(&element, "objects")?).map_err(data)?;
        members
            .iter()
            .map(|m| serde_json::from_value(hydrate(&db, m, "publication_id")?).map_err(data))
            .collect()
    }

    pub(in crate::world_state::graph) fn relations(
        &self,
        seq: u64,
        address: &DomainObjectRef,
        incoming: bool,
    ) -> Result<Vec<OwnerRelationOccurrence>, StorageError> {
        let db = self.db.read();
        let Some(node) = alias(&db, &address_alias(seq, address)?)? else {
            return Ok(Vec::new());
        };
        let search = if incoming {
            Q::search()
                .to(node.id)
                .where_()
                .distance(CountComparison::LessThanOrEqual(1))
                .query()
        } else {
            Q::search()
                .from(node.id)
                .where_()
                .distance(CountComparison::LessThanOrEqual(1))
                .query()
        };
        let elements = db
            .exec(Q::select().ids(search).query())
            .map_err(data)?
            .elements;
        elements
            .iter()
            .filter(|e| e.id.0 < 0)
            .map(|e| {
                let member: Member =
                    serde_json::from_str(property(e, "relation")?).map_err(data)?;
                serde_json::from_value(hydrate(&db, &member, "occurrence_id")?).map_err(data)
            })
            .collect()
    }
}

fn member(
    tx: &mut DbTransactionMut<'_>,
    cache: &mut BTreeMap<String, (i64, String)>,
    mut value: Value,
    identity: &str,
    ordinal: usize,
) -> Result<Member, StorageError> {
    let record = value
        .as_object_mut()
        .ok_or_else(|| data("record is not an object"))?;
    let id = record
        .remove(identity)
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| data("missing member identity"))?;
    let mut evidence = serde_json::Map::new();
    for field in ["source_product_ref", "hydration", "provenance_refs"] {
        evidence.insert(
            field.into(),
            record
                .remove(field)
                .ok_or_else(|| data("missing evidence field"))?,
        );
    }
    Ok(Member {
        id,
        content: intern(tx, cache, &value)?,
        evidence: intern(tx, cache, &Value::Object(evidence))?,
        ordinal,
    })
}

fn intern(
    tx: &mut DbTransactionMut<'_>,
    cache: &mut BTreeMap<String, (i64, String)>,
    value: &Value,
) -> Result<i64, StorageError> {
    let body = encode(value)?;
    let name = key("content", value)?;
    if let Some((id, stored)) = cache.get(&name) {
        if stored != &body {
            return Err(data("shared content identity collision"));
        }
        return Ok(*id);
    }
    let id = match tx.exec(Q::select().ids(name.as_str()).query()) {
        Ok(result) => {
            let element = &result.elements[0];
            if property(element, "body")? != body {
                return Err(data("shared content identity collision"));
            }
            element.id.0
        }
        Err(error) if error.ty == DbErrorType::NotFound => {
            tx.exec_mut(
                Q::insert()
                    .nodes()
                    .aliases(name.as_str())
                    .values_uniform([("body", body.clone()).into()])
                    .query(),
            )
            .map_err(data)?
            .elements[0]
                .id
                .0
        }
        Err(error) => return Err(data(error)),
    };
    cache.insert(name, (id, body));
    Ok(id)
}

fn hydrate(db: &Db, member: &Member, identity: &str) -> Result<Value, StorageError> {
    let mut result = serde_json::Map::new();
    for id in [member.content, member.evidence] {
        let element = db
            .exec(Q::select().ids(DbId(id)).query())
            .map_err(data)?
            .elements
            .into_iter()
            .next()
            .ok_or_else(|| data("missing shared content"))?;
        let body: serde_json::Map<String, Value> =
            serde_json::from_str(property(&element, "body")?).map_err(data)?;
        result.extend(body);
    }
    result.insert(identity.into(), Value::String(member.id.clone()));
    Ok(Value::Object(result))
}

fn reconstruct(db: &Db, element: &DbElement) -> Result<ProjectedOwnerPublication, StorageError> {
    let metadata = header(element)?;
    let mut objects = BTreeMap::new();
    let mut relations = BTreeMap::new();
    for element in db
        .exec(
            Q::select()
                .search()
                .index("revision-members")
                .value(metadata.source_event.seq.to_string())
                .query(),
        )
        .map_err(data)?
        .elements
    {
        if element.id.0 > 0 {
            let members: Vec<Member> =
                serde_json::from_str(property(&element, "objects")?).map_err(data)?;
            for member in members {
                objects.insert(
                    member.ordinal,
                    serde_json::from_value(hydrate(db, &member, "publication_id")?)
                        .map_err(data)?,
                );
            }
        } else {
            let member: Member =
                serde_json::from_str(property(&element, "relation")?).map_err(data)?;
            relations.insert(
                member.ordinal,
                serde_json::from_value(hydrate(db, &member, "occurrence_id")?).map_err(data)?,
            );
        }
    }
    let publication = ProjectedOwnerPublication {
        source_event: metadata.source_event,
        source_route: metadata.source_route,
        operation: OwnerPublicationOperation {
            operation_id: metadata.operation_id,
            enumeration_rule_revision: metadata.enumeration_rule_revision,
            batch: OwnerPublicationBatch {
                owner_id: metadata.owner_id,
                revision_id: metadata.revision_id,
                scope: metadata.scope,
                work_input_basis_id: metadata.work_input_basis_id,
                completeness: metadata.completeness,
                objects: objects.into_values().collect(),
                relations: relations.into_values().collect(),
            },
        },
    };
    if publication.operation.event_record_id() != metadata.event_record_id {
        return Err(data("header Event identity disagrees with operation"));
    }
    publication.operation.validate()?;
    Ok(publication)
}

struct TransactionError(StorageError);
impl From<agdb::DbError> for TransactionError {
    fn from(error: agdb::DbError) -> Self {
        Self(data(error))
    }
}
fn transact<T>(
    db: &mut Db,
    f: impl FnOnce(&mut DbTransactionMut<'_>) -> Result<T, StorageError>,
) -> Result<T, StorageError> {
    db.transaction_mut(|tx| f(tx).map_err(TransactionError))
        .map_err(|e| e.0)
}
