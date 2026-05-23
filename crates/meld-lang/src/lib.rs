//! Pure proposition and planning language contracts for Meld.

pub mod composition;
pub mod condition;
pub mod cost;
pub mod effect;
pub mod evaluate;
pub mod goal;
pub mod method;
pub mod operator;
pub mod proposition;
pub mod substitute;
pub mod term;
pub mod unify;
pub mod validate;
pub mod world_state;

pub use composition::{Composition, Edge, EdgeKind, Step, StepKind};
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
