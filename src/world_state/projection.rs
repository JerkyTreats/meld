#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CurrentClaimProjection {
    pub last_applied_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClaimProvenanceProjection {
    pub last_applied_seq: u64,
}
