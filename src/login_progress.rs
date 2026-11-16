//! Presentation follows SDK operation boundaries, never elapsed-time guesses.
use plicowarden_sdk::service::LoginStage;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Pending,
    Loading,
    Complete,
    Failed,
    Waiting,
    Paused,
}
#[derive(Clone, Debug)]
pub struct Step {
    pub stage: LoginStage,
    pub state: StepState,
}
#[derive(Clone, Default, Debug)]
pub struct LoginProgress {
    pub steps: Vec<Step>,
    pub visible: bool,
}
impl LoginProgress {
    pub fn begin(&mut self, cached: bool) {
        self.steps = vec![
            if cached {
                LoginStage::UnlockingCache
            } else {
                LoginStage::Authenticating
            },
            LoginStage::Syncing,
            LoginStage::Decrypting,
        ]
        .into_iter()
        .map(|stage| Step {
            stage,
            state: StepState::Pending,
        })
        .collect();
        self.visible = true;
        self.steps[0].state = StepState::Loading;
    }
    /// A manual or background refresh has one visible stage. The vault stays
    /// usable and the footer's sync glyph carries the live indication.
    pub fn begin_sync(&mut self) {
        self.steps = vec![Step {
            stage: LoginStage::Syncing,
            state: StepState::Loading,
        }];
        self.visible = true;
    }
    pub fn activate(&mut self, stage: LoginStage) {
        if self.steps.is_empty()
            || (stage == LoginStage::Authenticating
                && self.steps[0].stage == LoginStage::UnlockingCache)
            || (stage == LoginStage::UnlockingCache && self.steps[0].stage != stage)
        {
            self.begin(stage == LoginStage::UnlockingCache);
        }
        let Some(index) = self.steps.iter().position(|step| step.stage == stage) else {
            return;
        };
        for (i, step) in self.steps.iter_mut().enumerate() {
            if i < index {
                step.state = StepState::Complete;
            } else if i == index {
                step.state = StepState::Loading;
            }
        }
    }
    pub fn fail(&mut self) {
        if let Some(step) = self
            .steps
            .iter_mut()
            .find(|step| step.state == StepState::Loading)
        {
            step.state = StepState::Failed;
        }
    }
    pub fn verification(&mut self) {
        if let Some(step) = self
            .steps
            .iter_mut()
            .find(|step| step.stage == LoginStage::Authenticating)
        {
            step.state = StepState::Waiting;
        }
    }
    pub fn pause(&mut self) {
        if let Some(step) = self
            .steps
            .iter_mut()
            .find(|step| step.state == StepState::Loading)
        {
            step.state = StepState::Paused;
        }
    }
    pub fn finish(&mut self, local: bool, decrypt_failures: usize) {
        if local {
            if let Some(step) = self
                .steps
                .iter_mut()
                .find(|step| step.stage == LoginStage::UnlockingCache)
            {
                step.state = StepState::Complete;
            }
        } else {
            for step in &mut self.steps {
                step.state = if step.stage == LoginStage::Decrypting && decrypt_failures > 0 {
                    StepState::Failed
                } else {
                    StepState::Complete
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_preserves_completed_and_pending_steps() {
        let mut progress = LoginProgress::default();
        progress.begin(false);
        progress.activate(LoginStage::Syncing);
        progress.fail();
        assert_eq!(
            progress.steps.iter().map(|s| s.state).collect::<Vec<_>>(),
            [StepState::Complete, StepState::Failed, StepState::Pending]
        );
        progress.begin(false);
        assert_eq!(progress.steps[0].state, StepState::Loading);
        assert_eq!(progress.steps[1].state, StepState::Pending);
    }
    #[test]
    fn cache_unlock_does_not_claim_background_sync_finished() {
        let mut progress = LoginProgress::default();
        progress.begin(true);
        progress.finish(true, 0);
        assert_eq!(progress.steps[0].state, StepState::Complete);
        assert_eq!(progress.steps[1].state, StepState::Pending);
        progress.activate(LoginStage::Syncing);
        progress.pause();
        assert_eq!(progress.steps[1].state, StepState::Paused);
        progress.activate(LoginStage::Decrypting);
        progress.finish(false, 2);
        assert_eq!(progress.steps[1].state, StepState::Complete);
        assert_eq!(progress.steps[2].state, StepState::Failed);
    }
    #[test]
    fn verification_waits_without_marking_authentication_successful() {
        let mut progress = LoginProgress::default();
        progress.begin(false);
        progress.verification();
        assert_eq!(progress.steps[0].state, StepState::Waiting);
        progress.activate(LoginStage::Authenticating);
        assert_eq!(progress.steps[0].state, StepState::Loading);
        progress.activate(LoginStage::Syncing);
        assert_eq!(progress.steps[0].state, StepState::Complete);
    }
}
