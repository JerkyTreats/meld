//! Passive source delivery with subscription lineage and no execution attempt.

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassiveSubscriptionStatus {
    Active,
    Fenced,
    Retired,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassiveSubscriptionV1 {
    pub subscription_id: String,
    pub assignment_id: String,
    pub generation_id: String,
    pub participant_id: String,
    pub incarnation_id: String,
    pub source_id: String,
    pub admitted_cursor: String,
    pub status: PassiveSubscriptionStatus,
}
impl PassiveSubscriptionV1 {
    pub fn new(
        assignment_id: String,
        generation_id: String,
        participant_id: String,
        incarnation_id: String,
        source_id: String,
    ) -> Result<Self, DeliveryError> {
        let subscription_id = hash(&(
            &assignment_id,
            &generation_id,
            &participant_id,
            &incarnation_id,
            &source_id,
        ))?;
        Ok(Self {
            subscription_id,
            assignment_id,
            generation_id,
            participant_id,
            incarnation_id,
            source_id,
            admitted_cursor: "0".into(),
            status: PassiveSubscriptionStatus::Active,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassiveDeliveryV1 {
    pub delivery_id: String,
    pub subscription_id: String,
    pub incarnation_id: String,
    pub source_cursor: String,
    pub source_revision: String,
    pub payload_hash: String,
}
impl PassiveDeliveryV1 {
    pub fn new(
        subscription_id: String,
        incarnation_id: String,
        source_cursor: String,
        source_revision: String,
        payload_hash: String,
    ) -> Result<Self, DeliveryError> {
        let delivery_id = hash(&(
            &subscription_id,
            &incarnation_id,
            &source_cursor,
            &source_revision,
            &payload_hash,
        ))?;
        Ok(Self {
            delivery_id,
            subscription_id,
            incarnation_id,
            source_cursor,
            source_revision,
            payload_hash,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LateResultDisposition {
    Reject,
    HistoricalOnly,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryAdmission {
    Current,
    HistoricalOnly,
    Duplicate,
}
#[derive(Debug, Error)]
pub enum DeliveryError {
    #[error("delivery_invalid: {0}")]
    Invalid(String),
    #[error("delivery_conflict: {0}")]
    Conflict(String),
    #[error("delivery_storage: {0}")]
    Storage(String),
}

#[derive(Clone)]
pub struct PassiveDeliveryStore {
    db: Db,
    subscriptions: Tree,
    deliveries: Tree,
}
impl PassiveDeliveryStore {
    pub fn new(db: Db) -> Result<Self, DeliveryError> {
        Ok(Self {
            subscriptions: db
                .open_tree("pds_passive_subscriptions_v1")
                .map_err(storage)?,
            deliveries: db.open_tree("pds_passive_deliveries_v1").map_err(storage)?,
            db,
        })
    }
    pub fn subscribe(&self, item: PassiveSubscriptionV1) -> Result<(), DeliveryError> {
        put_immutable(&self.subscriptions, &item.subscription_id, &item)?;
        self.db.flush().map_err(storage)?;
        Ok(())
    }
    pub fn fence(&self, subscription_id: &str, retired: bool) -> Result<(), DeliveryError> {
        let mut item: PassiveSubscriptionV1 = get(&self.subscriptions, subscription_id)?;
        item.status = if retired {
            PassiveSubscriptionStatus::Retired
        } else {
            PassiveSubscriptionStatus::Fenced
        };
        replace(&self.subscriptions, subscription_id, &item)
    }
    pub fn admit(
        &self,
        delivery: PassiveDeliveryV1,
        late: LateResultDisposition,
    ) -> Result<DeliveryAdmission, DeliveryError> {
        let mut subscription: PassiveSubscriptionV1 =
            get(&self.subscriptions, &delivery.subscription_id)?;
        if subscription.incarnation_id != delivery.incarnation_id {
            return Err(DeliveryError::Conflict(
                "delivery incarnation is fenced".into(),
            ));
        }
        if self
            .deliveries
            .get(delivery.delivery_id.as_bytes())
            .map_err(storage)?
            .is_some()
        {
            return Ok(DeliveryAdmission::Duplicate);
        }
        if !matches!(subscription.status, PassiveSubscriptionStatus::Active) {
            return match late {
                LateResultDisposition::Reject => {
                    Err(DeliveryError::Conflict("late delivery rejected".into()))
                }
                LateResultDisposition::HistoricalOnly => {
                    put_immutable(&self.deliveries, &delivery.delivery_id, &delivery)?;
                    Ok(DeliveryAdmission::HistoricalOnly)
                }
            };
        }
        let prior: u64 = subscription
            .admitted_cursor
            .parse()
            .map_err(|_| DeliveryError::Invalid("stored source cursor is invalid".into()))?;
        let next: u64 = delivery
            .source_cursor
            .parse()
            .map_err(|_| DeliveryError::Invalid("source cursor is invalid".into()))?;
        if next <= prior {
            return Err(DeliveryError::Conflict(
                "source cursor did not advance".into(),
            ));
        }
        put_immutable(&self.deliveries, &delivery.delivery_id, &delivery)?;
        subscription.admitted_cursor = delivery.source_cursor;
        replace(
            &self.subscriptions,
            &subscription.subscription_id,
            &subscription,
        )?;
        self.db.flush().map_err(storage)?;
        Ok(DeliveryAdmission::Current)
    }
    pub fn delivery_count(&self) -> Result<usize, DeliveryError> {
        Ok(self.deliveries.len())
    }
}
fn put_immutable<T: Serialize>(tree: &Tree, key: &str, value: &T) -> Result<(), DeliveryError> {
    let bytes = bincode::serialize(value).map_err(|e| DeliveryError::Storage(e.to_string()))?;
    if let Some(existing) = tree.get(key.as_bytes()).map_err(storage)? {
        if existing.as_ref() != bytes.as_slice() {
            return Err(DeliveryError::Conflict(
                "immutable delivery record differs".into(),
            ));
        }
    } else {
        tree.insert(key.as_bytes(), bytes).map_err(storage)?;
    }
    Ok(())
}
fn replace<T: Serialize>(tree: &Tree, key: &str, value: &T) -> Result<(), DeliveryError> {
    tree.insert(
        key.as_bytes(),
        bincode::serialize(value).map_err(|e| DeliveryError::Storage(e.to_string()))?,
    )
    .map_err(storage)?;
    Ok(())
}
fn get<T: serde::de::DeserializeOwned>(tree: &Tree, key: &str) -> Result<T, DeliveryError> {
    let bytes = tree
        .get(key.as_bytes())
        .map_err(storage)?
        .ok_or_else(|| DeliveryError::Invalid("subscription is absent".into()))?;
    bincode::deserialize(&bytes).map_err(|e| DeliveryError::Storage(e.to_string()))
}
fn hash(value: &impl Serialize) -> Result<String, DeliveryError> {
    bincode::serialize(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|e| DeliveryError::Storage(e.to_string()))
}
fn storage(error: sled::Error) -> DeliveryError {
    DeliveryError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passive_delivery_creates_no_execution_attempt() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = PassiveDeliveryStore::new(db.clone()).unwrap();
        let subscription = PassiveSubscriptionV1::new(
            "a".into(),
            "g".into(),
            "p".into(),
            "i".into(),
            "fixture".into(),
        )
        .unwrap();
        store.subscribe(subscription.clone()).unwrap();
        let delivery = PassiveDeliveryV1::new(
            subscription.subscription_id,
            "i".into(),
            "1".into(),
            "r2".into(),
            "payload".into(),
        )
        .unwrap();
        assert_eq!(
            store
                .admit(delivery, LateResultDisposition::Reject)
                .unwrap(),
            DeliveryAdmission::Current
        );
        assert_eq!(store.delivery_count().unwrap(), 1);
        assert_eq!(db.open_tree("pds_operation_attempts_v1").unwrap().len(), 0);
    }
    #[test]
    fn retired_generation_delivery_never_advances_current() {
        let store =
            PassiveDeliveryStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        let subscription = PassiveSubscriptionV1::new(
            "a".into(),
            "old".into(),
            "p".into(),
            "i".into(),
            "fixture".into(),
        )
        .unwrap();
        store.subscribe(subscription.clone()).unwrap();
        store.fence(&subscription.subscription_id, true).unwrap();
        let delivery = PassiveDeliveryV1::new(
            subscription.subscription_id,
            "i".into(),
            "1".into(),
            "r2".into(),
            "payload".into(),
        )
        .unwrap();
        assert_eq!(
            store
                .admit(delivery, LateResultDisposition::HistoricalOnly)
                .unwrap(),
            DeliveryAdmission::HistoricalOnly
        );
    }
}
