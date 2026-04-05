#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    BuildProjection,
    EmbedContent,
    SyncBatch,
}

pub fn default_poll_interval_secs() -> u64 {
    5
}

pub fn queue_name(kind: JobKind) -> &'static str {
    match kind {
        JobKind::BuildProjection => "projection",
        JobKind::EmbedContent => "embedding",
        JobKind::SyncBatch => "sync",
    }
}

#[cfg(test)]
mod tests {
    use super::{JobKind, default_poll_interval_secs, queue_name};

    #[test]
    fn maps_job_kind_to_queue() {
        assert_eq!(queue_name(JobKind::SyncBatch), "sync");
    }

    #[test]
    fn exposes_default_poll_interval() {
        assert_eq!(default_poll_interval_secs(), 5);
    }
}
