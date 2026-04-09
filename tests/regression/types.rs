use uuid::Uuid;

pub type ArtifactId = Uuid;
pub type ExecutorId = &'static str;
pub type FindingId = Uuid;
pub type RunId = Uuid;
pub type TransitionId = &'static str;
pub type ValidatorId = &'static str;
