//! Native products supplied to configured owners during exact preparation.

use super::{OwnerDiagnosticV1, OwnerObservationPreparationV1};
use crate::capability::OwnerBindingView;
use crate::config::PhysicalBinding;
use crate::runtime::storage::OpenProductStores;
use crate::theory::InstalledTheoryComponentRef;

pub(crate) fn prepare_product_owner_bindings(
    stores: &OpenProductStores,
    binding: &PhysicalBinding,
    positions: &[crate::runtime::BoundProductPosition],
) -> Result<OwnerBindingView, OwnerDiagnosticV1> {
    let views = positions
        .iter()
        .map(|position| prepare_owner_bindings(stores, &position.physical, &position.components))
        .collect::<Result<Vec<_>, _>>()?;
    if binding.agent_positions.is_empty() {
        return views
            .into_iter()
            .next()
            .ok_or_else(|| failure("product has no bound Agent position"));
    }
    stores.owners.compose_position_bindings(
        OwnerBindingView::new(binding.owner_binding_values()),
        &views,
    )
}

pub fn prepare_owner_bindings(
    stores: &OpenProductStores,
    binding: &PhysicalBinding,
    components: &[InstalledTheoryComponentRef],
) -> Result<OwnerBindingView, OwnerDiagnosticV1> {
    if stores.owners.is_empty() {
        return Ok(OwnerBindingView::new(binding.owner_binding_values()));
    }
    let authority = components
        .iter()
        .find(|component| {
            component.route.owner_domain == "execution"
                && component.route.component_kind == "authority-policy"
                && component.owner_revision.id == binding.package.authority_policy_id
        })
        .ok_or_else(|| failure("selected owner authority revision is absent"))?;
    let authority = stores
        .authority_policy_registry
        .resolve(
            &authority.owner_revision.id,
            &authority.owner_revision.content_hash,
        )
        .map_err(failure)?
        .ok_or_else(|| failure("selected authority is not installed"))?
        .binding()
        .map_err(failure)?;
    let event_routes = stores
        .traversal_store
        .owner_event_routes()
        .map_err(failure)?
        .into_iter()
        .filter(|route| {
            route.revision_ref().is_ok_and(|reference| {
                components.iter().any(|component| {
                    component.owner_revision.registry == reference.registry
                        && component.owner_revision.id == reference.id
                        && component.owner_revision.content_hash == reference.content_hash
                })
            })
        })
        .collect();
    stores.owners.prepare_bindings(
        binding,
        components,
        &OwnerObservationPreparationV1 {
            subject: binding.subject.clone(),
            scope: meld_world_model::world_state::graph::contracts::OwnerPublicationScope {
                scope_id: binding.assignment_scope_id(),
                branch_id: Some("main".into()),
                perspective_id: Some("default".into()),
                valid_at: None,
            },
            session_id: format!("stewardship::{}", binding.assignment_scope_id()),
            authority,
            event_routes,
        },
    )
}
fn failure(error: impl ToString) -> OwnerDiagnosticV1 {
    OwnerDiagnosticV1::new("owner_preparation_invalid", error)
}
