//! Pure proposition and planning language contracts for Meld.
//!
//! This crate owns the typed language shared by goal planning, world-state
//! evaluation, operator resolution, composition validation, unification, and
//! substitution.
//!
//! This crate does not own workflow orchestration, capability execution,
//! durable event storage, or world-model materialization. Those domains consume
//! these contracts through explicit propositions, effects, goals, and
//! compositions.
//!
//! # Module Map
//!
//! - [`composition`] defines step graphs and typed dependency edges.
//! - [`authority`] defines pure effective-authority policy and decision values.
//! - [`proposition`] and [`term`] define the ground and pattern language.
//! - [`mod@evaluate`] and [`world_state`] evaluate propositions against belief.
//! - [`mod@unify`] and [`mod@substitute`] bind and instantiate language patterns.
//! - [`mod@validate`] checks composition structure before execution.
//!
//! Start with [`Proposition`] and [`Term`] when modeling facts, [`WorldState`]
//! when evaluating belief, and [`Composition`] when representing a planned
//! operator graph.
//!
//! # Example
//!
//! ```rust
//! use meld_lang::{Condition, Literal, Proposition, Term, WorldState, evaluate};
//!
//! let node = Term::Object(
//!     meld_events::DomainObjectRef::new("world", "node", "node-a").unwrap(),
//! );
//! let proposition = Proposition::Holds {
//!     subject: node.clone(),
//!     dimension: Term::Dimension("confidence".to_string()),
//!     condition: Condition::Equals(Term::Literal(Literal::Number(0.9))),
//! };
//! let state = WorldState::new(vec![proposition.clone()]).unwrap();
//!
//! assert!(state.satisfies(&proposition));
//! assert_eq!(evaluate(&state, &proposition), meld_lang::EvalResult::Satisfied);
//! ```

#![deny(missing_docs)]

/// Pure effective-authority policy, decision, and evaluation contracts.
pub mod authority;
/// Planning step graphs and typed dependency edges.
pub mod composition;
/// Proposition condition operators.
pub mod condition;
/// Resource cost estimates and aggregation helpers.
pub mod cost;
/// Deterministic effects over proposition state.
pub mod effect;
/// Three-valued proposition evaluation.
pub mod evaluate;
/// Goal contracts and lifecycle metadata.
pub mod goal;
/// Reusable planning method templates.
pub mod method;
/// Runtime operator and capability resolution contracts.
pub mod operator;
/// Typed statements about the world.
pub mod proposition;
/// Variable substitution over compositions.
pub mod substitute;
/// Atomic language terms and literal values.
pub mod term;
/// Proposition unification and variable bindings.
pub mod unify;
/// Structural composition validation.
pub mod validate;
/// Ground proposition sets and effect application.
pub mod world_state;

pub use authority::{
    evaluate_authority, required_action_ids, AuthorityDecision, AuthorityDenial, AuthorityPolicy,
    AuthorityPolicyBinding,
};
pub use composition::{Composition, Edge, EdgeKind, Step, StepKind, TaskInput};
pub use condition::Condition;
pub use cost::CostEstimate;
pub use effect::Effect;
pub use evaluate::{evaluate, EvalResult};
pub use goal::{Goal, GoalLifecycle, GoalPriority, GoalSource};
pub use method::Method;
pub use operator::{CapabilityRef, Operator, Resolution, SlotConstraint};
pub use proposition::Proposition;
pub use substitute::{substitute, SubstitutionError, UnboundVariable};
pub use term::{Literal, Term};
pub use unify::{unify, Bindings};
pub use validate::{validate, ValidationError, ValidationResult, ValidationWarning};
pub use world_state::{GroundingError, WorldState};
