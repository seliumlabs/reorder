pub struct ArtifactId(pub String);

pub struct TransitionId(pub String);

pub struct ArtifactRef {
    data: i32,
}

impl ArtifactId {
    pub fn new() -> Self {
        Self(String::new())
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

impl TransitionId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
}

impl ArtifactRef {
    pub fn downcast_ref(&self) -> i32 {
        self.data
    }
}
