//! Exact situated Agent contexts selected from a compiled product.

use super::storage::OpenProductStores;
use crate::config::PhysicalBinding;
use crate::error::ApiError;
use crate::theory::{
    InstalledTheoryComponentRef, ProductCompilationReceiptV1, ProductDeclarationV1,
};

pub(crate) fn prepared_product_positions(
    stores: &OpenProductStores,
    binding: &PhysicalBinding,
) -> Result<
    Option<(
        crate::theory::PreparedActivationClosureV1,
        Vec<BoundProductPosition>,
    )>,
    ApiError,
> {
    let Some(head) = stores
        .pds_products
        .prepared_head(&binding.package.expression, &binding.assignment_scope_id())
        .map_err(|e| ApiError::ConfigError(e.to_string()))?
    else {
        return Ok(None);
    };
    let closure = stores
        .pds_products
        .prepared_closure(&head.prepared_id)
        .map_err(|e| ApiError::ConfigError(e.to_string()))?
        .ok_or_else(|| ApiError::ConfigError("prepared product closure is absent".into()))?;
    let declaration = stores
        .pds_products
        .declaration(&closure.product_revision_id)
        .map_err(|e| ApiError::ConfigError(e.to_string()))?
        .ok_or_else(|| ApiError::ConfigError("prepared product declaration is absent".into()))?;
    let compilation = stores
        .pds_products
        .compilation(&closure.product_compilation_receipt_id)
        .map_err(|e| ApiError::ConfigError(e.to_string()))?
        .ok_or_else(|| ApiError::ConfigError("prepared product compilation is absent".into()))?;
    if binding.assigned_positions(&declaration.agent_topology)?
        != closure.assignment.agent_positions
        || closure.assignment.assignment_id != head.assignment_id
    {
        return Err(ApiError::ConfigError(
            "physical Agent bindings differ from the prepared assignment".into(),
        ));
    }
    let positions = bind_product_positions(stores, binding, &declaration, &compilation)?;
    Ok(Some((closure, positions)))
}

pub(crate) struct BoundProductPosition {
    pub position_id: String,
    pub physical: PhysicalBinding,
    pub components: Vec<InstalledTheoryComponentRef>,
}

pub(crate) fn bind_product_positions(
    stores: &OpenProductStores,
    binding: &PhysicalBinding,
    declaration: &ProductDeclarationV1,
    compilation: &ProductCompilationReceiptV1,
) -> Result<Vec<BoundProductPosition>, ApiError> {
    binding
        .assigned_positions(&declaration.agent_topology)?
        .into_iter()
        .map(|assigned| {
            let position = declaration
                .agent_topology
                .iter()
                .find(|p| p.position_id == assigned.position_id)
                .ok_or_else(|| {
                    ApiError::ConfigError("bound Agent position is absent from the product".into())
                })?;
            let selection = if binding.agent_positions.is_empty() {
                binding.package.clone()
            } else {
                compilation
                    .position_selection(position, declaration, &stores.pds_packages)
                    .map_err(|e| ApiError::ConfigError(e.to_string()))?
            };
            let components = compilation
                .position_packages(position, &stores.pds_packages)
                .map_err(|e| ApiError::ConfigError(e.to_string()))?
                .into_iter()
                .flat_map(|package| package.components)
                .collect();
            Ok(BoundProductPosition {
                position_id: assigned.position_id,
                physical: binding.for_agent(&assigned.agent_id, selection),
                components,
            })
        })
        .collect()
}
