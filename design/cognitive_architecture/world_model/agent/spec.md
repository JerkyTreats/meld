# Agent Contract

An Agent has stable identity, one PDS-derived directive context, an activation generation, admitted Goals, authorized Plans, durable progress records, and input cursors.

Agent decisions are immutable facts. A later decision may supersede an earlier one but cannot rewrite it.

Authorization binds Agent, Goal, Plan revision, exact product identity, frozen context identity, authority scope, and idempotency key.

Agent may declare Goal satisfaction only through the Goal's owning satisfaction semantics and admitted evidence. Task completion alone and Epistemic Operation completion alone are never universal satisfaction proofs.
