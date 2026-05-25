use super::LogIndex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplicationRecoveryPolicy {
    pub snapshot_threshold: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicationRecoveryAction {
    AlreadyCaughtUp,
    AppendEntries {
        from_exclusive: LogIndex,
        to_inclusive: LogIndex,
    },
    InstallSnapshot {
        checkpoint: LogIndex,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplicationRecoveryPlan {
    pub lag: u64,
    pub action: ReplicationRecoveryAction,
}

impl Default for ReplicationRecoveryPolicy {
    fn default() -> Self {
        Self {
            snapshot_threshold: 1_024,
        }
    }
}

pub fn plan_replication_recovery(
    follower_commit: LogIndex,
    leader_commit: LogIndex,
    policy: ReplicationRecoveryPolicy,
) -> ReplicationRecoveryPlan {
    let lag = leader_commit.0.saturating_sub(follower_commit.0);
    let action = if lag == 0 {
        ReplicationRecoveryAction::AlreadyCaughtUp
    } else if lag > policy.snapshot_threshold {
        ReplicationRecoveryAction::InstallSnapshot {
            checkpoint: leader_commit,
        }
    } else {
        ReplicationRecoveryAction::AppendEntries {
            from_exclusive: follower_commit,
            to_inclusive: leader_commit,
        }
    };
    ReplicationRecoveryPlan { lag, action }
}
