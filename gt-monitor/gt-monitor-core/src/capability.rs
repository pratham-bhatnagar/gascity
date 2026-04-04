use serde::{Deserialize, Serialize};

/// A queryable data category. Each provider advertises which capabilities it supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    // Git platform
    Repos,
    Issues,
    PullRequests,
    Commits,
    Branches,
    Users,

    // Work tracking (beads)
    Beads,
    BeadEvents,
    Dependencies,
    Convoys,
    MergeQueue,

    // Agent system
    Agents,
    Sessions,
    Mail,
    Hooks,
    Scheduler,

    // Operations
    Costs,
    CommandMetrics,
    Changelog,

    // Federation
    WastelandItems,
    WastelandStamps,
    WastelandCharsheets,

    // Infrastructure
    SystemCpu,
    SystemMemory,
    SystemDisk,
    SystemProcesses,

    // Observability
    OtelMetrics,
    OtelLogs,
    OtelTraces,

    // Configuration
    AgentConfigs,
    RigConfigs,
    TownConfig,
    Formulas,
    Crontabs,
    KnowledgeBase,

    // Diagnostics
    LogFiles,
    BackupState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_serializes_to_snake_case() {
        let json = serde_json::to_string(&Capability::PullRequests).unwrap();
        assert_eq!(json, r#""pull_requests""#);
    }

    #[test]
    fn capability_deserializes_from_snake_case() {
        let cap: Capability = serde_json::from_str(r#""bead_events""#).unwrap();
        assert_eq!(cap, Capability::BeadEvents);
    }

    #[test]
    fn capability_roundtrips() {
        let caps = vec![
            Capability::Repos,
            Capability::Beads,
            Capability::SystemCpu,
            Capability::OtelMetrics,
            Capability::WastelandItems,
        ];
        for cap in caps {
            let json = serde_json::to_string(&cap).unwrap();
            let back: Capability = serde_json::from_str(&json).unwrap();
            assert_eq!(cap, back);
        }
    }
}
