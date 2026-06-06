use super::*;

impl UnauthorizedRecovery {
    pub(super) fn new(manager: Arc<AuthManager>) -> Self {
        let cached_auth = manager.auth_cached();
        let expected_account_id = cached_auth.as_ref().and_then(VACAuth::get_account_id);
        let mode = if manager.has_external_api_key_auth()
            || cached_auth
                .as_ref()
                .is_some_and(VACAuth::is_external_chatgpt_tokens)
        {
            UnauthorizedRecoveryMode::External
        } else {
            UnauthorizedRecoveryMode::Managed
        };
        let step = match mode {
            UnauthorizedRecoveryMode::Managed => UnauthorizedRecoveryStep::Reload,
            UnauthorizedRecoveryMode::External => UnauthorizedRecoveryStep::ExternalRefresh,
        };
        Self {
            manager,
            step,
            expected_account_id,
            mode,
        }
    }

    pub fn has_next(&self) -> bool {
        if self.manager.has_external_api_key_auth() {
            return !matches!(self.step, UnauthorizedRecoveryStep::Done);
        }

        if !self
            .manager
            .auth_cached()
            .as_ref()
            .is_some_and(VACAuth::is_chatgpt_auth)
        {
            return false;
        }

        if self.mode == UnauthorizedRecoveryMode::External && !self.manager.has_external_auth() {
            return false;
        }

        !matches!(self.step, UnauthorizedRecoveryStep::Done)
    }

    pub fn unavailable_reason(&self) -> &'static str {
        if self.manager.has_external_api_key_auth() {
            return if matches!(self.step, UnauthorizedRecoveryStep::Done) {
                "recovery_exhausted"
            } else {
                "ready"
            };
        }

        if !self
            .manager
            .auth_cached()
            .as_ref()
            .is_some_and(VACAuth::is_chatgpt_auth)
        {
            return "not_chatgpt_auth";
        }

        if self.mode == UnauthorizedRecoveryMode::External && !self.manager.has_external_auth() {
            return "no_external_auth";
        }

        if matches!(self.step, UnauthorizedRecoveryStep::Done) {
            return "recovery_exhausted";
        }

        "ready"
    }

    pub fn mode_name(&self) -> &'static str {
        match self.mode {
            UnauthorizedRecoveryMode::Managed => "managed",
            UnauthorizedRecoveryMode::External => "external",
        }
    }

    pub fn step_name(&self) -> &'static str {
        match self.step {
            UnauthorizedRecoveryStep::Reload => "reload",
            UnauthorizedRecoveryStep::RefreshToken => "refresh_token",
            UnauthorizedRecoveryStep::ExternalRefresh => "external_refresh",
            UnauthorizedRecoveryStep::Done => "done",
        }
    }

    pub async fn next(&mut self) -> Result<UnauthorizedRecoveryStepResult, RefreshTokenError> {
        if !self.has_next() {
            return Err(RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                RefreshTokenFailedReason::Other,
                "No more recovery steps available.",
            )));
        }

        match self.step {
            UnauthorizedRecoveryStep::Reload => {
                match self
                    .manager
                    .reload_if_account_id_matches(self.expected_account_id.as_deref())
                    .await
                {
                    ReloadOutcome::ReloadedChanged => {
                        self.step = UnauthorizedRecoveryStep::RefreshToken;
                        return Ok(UnauthorizedRecoveryStepResult {
                            auth_state_changed: Some(true),
                        });
                    }
                    ReloadOutcome::ReloadedNoChange => {
                        self.step = UnauthorizedRecoveryStep::RefreshToken;
                        return Ok(UnauthorizedRecoveryStepResult {
                            auth_state_changed: Some(false),
                        });
                    }
                    ReloadOutcome::Skipped => {
                        self.step = UnauthorizedRecoveryStep::Done;
                        return Err(RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                            RefreshTokenFailedReason::Other,
                            REFRESH_TOKEN_ACCOUNT_MISMATCH_MESSAGE.to_string(),
                        )));
                    }
                }
            }
            UnauthorizedRecoveryStep::RefreshToken => {
                self.manager.refresh_token_from_authority().await?;
                self.step = UnauthorizedRecoveryStep::Done;
                return Ok(UnauthorizedRecoveryStepResult {
                    auth_state_changed: Some(true),
                });
            }
            UnauthorizedRecoveryStep::ExternalRefresh => {
                self.manager
                    .refresh_external_auth(ExternalAuthRefreshReason::Unauthorized)
                    .await?;
                self.step = UnauthorizedRecoveryStep::Done;
                return Ok(UnauthorizedRecoveryStepResult {
                    auth_state_changed: Some(true),
                });
            }
            UnauthorizedRecoveryStep::Done => {}
        }
        Ok(UnauthorizedRecoveryStepResult {
            auth_state_changed: None,
        })
    }
}
