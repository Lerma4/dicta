use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Preparing,
    Recording,
    Processing,
    Inserting,
    Completed,
    Cancelled,
    Error,
}

impl Phase {
    pub fn transition(self, next: Self) -> Result<Self, String> {
        use Phase::*;
        let valid = matches!(
            (self, next),
            (Idle | Completed | Cancelled | Error, Preparing)
                | (Preparing, Recording)
                | (Recording, Processing)
                | (Processing, Inserting | Completed)
                | (Inserting, Completed)
                | (
                    Preparing | Recording | Processing | Inserting,
                    Error | Cancelled
                )
                | (Completed | Cancelled | Error, Idle)
        );
        if valid {
            Ok(next)
        } else {
            Err(format!("Invalid dictation transition: {self:?} → {next:?}"))
        }
    }

    pub fn active(self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Recording | Self::Processing | Self::Inserting
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub state: Phase,
    pub session_id: Option<String>,
    pub elapsed_ms: u64,
    pub level: f32,
    pub speech: bool,
    pub message: Option<String>,
    pub recovery_text: Option<String>,
    pub recovery_pending: bool,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            state: Phase::Idle,
            session_id: None,
            elapsed_ms: 0,
            level: 0.0,
            speech: false,
            message: None,
            recovery_text: None,
            recovery_pending: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn happy_path_is_linear_and_insertion_cannot_repeat() {
        let mut phase = Phase::Idle;
        for next in [
            Phase::Preparing,
            Phase::Recording,
            Phase::Processing,
            Phase::Inserting,
            Phase::Completed,
            Phase::Idle,
        ] {
            phase = phase.transition(next).unwrap();
        }
        assert!(Phase::Completed.transition(Phase::Inserting).is_err());
        assert!(Phase::Recording.transition(Phase::Recording).is_err());
        assert!(Phase::Idle.transition(Phase::Processing).is_err());
    }
    #[test]
    fn every_active_phase_can_be_cancelled_or_fail() {
        for phase in [
            Phase::Preparing,
            Phase::Recording,
            Phase::Processing,
            Phase::Inserting,
        ] {
            assert_eq!(
                phase.transition(Phase::Cancelled).unwrap(),
                Phase::Cancelled
            );
            assert_eq!(phase.transition(Phase::Error).unwrap(), Phase::Error);
        }
    }
}
