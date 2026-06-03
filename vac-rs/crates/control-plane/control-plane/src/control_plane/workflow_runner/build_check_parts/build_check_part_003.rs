impl WorkflowExecutionMachine {
    pub fn new(manifest: &WorkflowManifest) -> Self {
        let step_resolutions = manifest
            .steps
            .iter()
            .map(resolve_workflow_step)
            .collect::<Vec<_>>();
        Self {
            workflow_id: manifest.id.clone(),
            run_id: Uuid::new_v4().to_string(),
            attempt_id: "attempt-1".to_string(),
            title: manifest.title.clone(),
            step_count: manifest.steps.len(),
            step_resolutions,
            approval_checks: Vec::new(),
            policy_decisions: Vec::new(),
            approval_store: InMemoryApprovalStore::new(),
            persistent_approval_store: None,
            approval_requests: Vec::new(),
            identity_check_report: None,
            no_duplicate_tui_report: None,
            architecture_invariants_report: None,
            ownership_scan_report: None,
            root_seed_registry_report: None,
            build_check_report: None,
            build_check_repo_root: None,
            donor_status_report: None,
            build_check_cargo_program: None,
            tui_pty_gate_result: None,
            cursor: 0,
            started_step_count: 0,
            completed_step_count: 0,
            waiting_approval_step_count: 0,
            blocked_step_count: 0,
            started: false,
            finished: false,
            cancelled_reason: None,
            failed_reason: None,
            current_step_index: None,
            current_step_resolution: None,
            current_step_lifecycle: None,
            state_trace: vec![WorkflowExecutionState::Ready {
                workflow_id: manifest.id.clone(),
                title: manifest.title.clone(),
                step_count: manifest.steps.len(),
            }],
            events: Vec::new(),
        }
    }

    pub fn with_run_attempt_id(
        mut self,
        run_id: impl Into<String>,
        attempt_id: impl Into<String>,
    ) -> Self {
        self.run_id = run_id.into();
        self.attempt_id = attempt_id.into();
        self
    }

    pub fn state_trace(&self) -> &[WorkflowExecutionState] {
        &self.state_trace
    }

    pub fn events(&self) -> &[WorkflowExecutionEvent] {
        &self.events
    }

    pub fn approval_requests(&self) -> &[ApprovalRequest] {
        &self.approval_requests
    }

    pub fn with_persistent_approval_store(mut self, store: FileApprovalStore) -> Self {
        self.persistent_approval_store = Some(store);
        self
    }

    pub fn with_approval_checks(mut self, approval_checks: Vec<WorkflowApprovalCheck>) -> Self {
        self.approval_checks = approval_checks;
        self
    }

    pub fn with_identity_check_report(mut self, report: IdentityCheckReport) -> Self {
        self.identity_check_report = Some(report);
        self
    }

    pub fn with_no_duplicate_tui_report(mut self, report: NoDuplicateTuiReport) -> Self {
        self.no_duplicate_tui_report = Some(report);
        self
    }

    pub fn with_architecture_invariants_report(
        mut self,
        report: ArchitectureInvariantReport,
    ) -> Self {
        self.architecture_invariants_report = Some(report);
        self
    }

    pub fn with_ownership_scan_report(mut self, report: OwnershipScanReport) -> Self {
        self.ownership_scan_report = Some(report);
        self
    }

    pub fn with_root_seed_registry_report(mut self, report: RegistryLoadReport) -> Self {
        self.root_seed_registry_report = Some(report);
        self
    }

    pub fn with_build_check_report(mut self, report: build_check::BuildCheckReport) -> Self {
        self.build_check_report = Some(report);
        self
    }

    pub fn with_build_check_repo_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.build_check_repo_root = Some(root.into());
        self
    }

    pub fn with_donor_status_report(mut self, report: DonorStatusReport) -> Self {
        self.donor_status_report = Some(report);
        self
    }

    pub fn with_build_check_cargo_program(mut self, cargo: impl Into<PathBuf>) -> Self {
        self.build_check_cargo_program = Some(cargo.into());
        self
    }

    pub fn with_tui_pty_gate_result(mut self, result: TuiPtyGateResult) -> Self {
        self.tui_pty_gate_result = Some(result);
        self
    }

    pub fn with_policy_decisions(
        mut self,
        decisions: Vec<Option<super::policy_manifest::PolicyDecisionReport>>,
    ) -> Self {
        self.policy_decisions = decisions;
        self
    }

    pub fn state(&self) -> WorkflowExecutionState {
        if let Some(reason) = &self.cancelled_reason {
            WorkflowExecutionState::Cancelled {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                reason: reason.clone(),
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            }
        } else if let Some(reason) = &self.failed_reason {
            WorkflowExecutionState::Failed {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                reason: reason.clone(),
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            }
        } else if self.finished {
            WorkflowExecutionState::Finished {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            }
        } else if let (Some(index), Some(resolution), Some(lifecycle)) = (
            self.current_step_index,
            self.current_step_resolution.clone(),
            self.current_step_lifecycle,
        ) {
            WorkflowExecutionState::Step {
                index,
                step_count: self.step_count,
                resolution,
                lifecycle,
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            }
        } else {
            WorkflowExecutionState::Ready {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                step_count: self.step_count,
            }
        }
    }

    fn request_approval_for_step(
        &mut self,
        resolution: &WorkflowStepResolution,
        reasons: Vec<String>,
    ) -> Result<ApprovalRequestId, super::approval_lifecycle::ApprovalStoreError> {
        let request_id = ApprovalRequestId::for_workflow_step_attempt(
            &self.workflow_id,
            &self.run_id,
            &self.attempt_id,
            &resolution.id,
        );
        let (action, risk) = approval_action_and_risk_for_step(&resolution.uses);
        let request = ApprovalRequest::pending(
            request_id.clone(),
            self.workflow_id.clone(),
            resolution.id.clone(),
            resolve_workflow_approval_capability_id(&resolution.uses)
                .unwrap_or(resolution.uses.as_str())
                .to_string(),
            action,
            risk,
            reasons,
            Utc::now(),
            Some(Utc::now() + Duration::minutes(30)),
        );
        self.approval_store.request(request.clone());
        if let Some(store) = self.persistent_approval_store.as_mut() {
            store.request(request.clone())?;
        }
        self.approval_requests.push(request);
        Ok(request_id)
    }

    fn sync_resolved_approval_request(&mut self, resolved: ApprovalRequest) {
        if let Some(request) = self
            .approval_requests
            .iter_mut()
            .find(|request| request.id == resolved.id)
        {
            *request = resolved;
        }
    }

    fn persist_resolved_approval_request(
        &mut self,
        id: &ApprovalRequestId,
        decision: ApprovalDecision,
    ) -> Result<(), super::approval_lifecycle::ApprovalStoreError> {
        if let Some(store) = self.persistent_approval_store.as_mut() {
            store.resolve(id, decision)?;
        }
        Ok(())
    }

    fn approval_resolution_reason(request: &ApprovalRequest) -> String {
        request
            .decision_reason
            .clone()
            .unwrap_or_else(|| format!("approval {}", request.status))
    }

    fn policy_block_reason_for_step(&self, index: usize) -> Option<String> {
        match self.policy_decisions.get(index.saturating_sub(1)) {
            Some(Some(super::policy_manifest::PolicyDecisionReport::Block { reasons })) => {
                Some(format!("policy: {}", reasons.join("; ")))
            }
            _ => None,
        }
    }

    fn unimplemented_handler_reason(resolution: &WorkflowStepResolution) -> Option<String> {
        match resolution.handler? {
            WorkflowStepHandler::ActivityEmit
            | WorkflowStepHandler::DonorMigrationInventoryCheck
            | WorkflowStepHandler::DonorMigrationDriftCheck
            | WorkflowStepHandler::DonorMigrationManifestCheck
            | WorkflowStepHandler::DonorMigrationReachabilityCheck
            | WorkflowStepHandler::DonorMigrationEvidenceCheck
            | WorkflowStepHandler::DonorMigrationCommitPhraseCheck
            | WorkflowStepHandler::RootSeedCoverage
            | WorkflowStepHandler::BuildCargoCheck
            | WorkflowStepHandler::IdentityCheck
            | WorkflowStepHandler::NoDuplicateTui
            | WorkflowStepHandler::OwnershipScan
            | WorkflowStepHandler::ArchitectureInvariants
            | WorkflowStepHandler::TuiPtyGate
            | WorkflowStepHandler::ApprovalRequest
            | WorkflowStepHandler::RegistrySchemaCheck
            | WorkflowStepHandler::CapabilityDashboardCheck
            | WorkflowStepHandler::WorkflowBrowserCheck => None,
        }
    }

    fn build_check_failure_reason(report: &build_check::BuildCheckReport) -> String {
        if report.timed_out {
            return "build check timed out".to_string();
        }
        if let Some(diagnostic) = report.diagnostics.first() {
            return diagnostic.clone();
        }
        if let Some(stderr) = report.stderr_summary.first() {
            return stderr.clone();
        }
        format!(
            "build check failed exit_status={}",
            report
                .exit_status
                .map(|status| status.to_string())
                .unwrap_or_else(|| "signal".to_string())
        )
    }

    fn execute_build_cargo_check(
        &mut self,
        index: usize,
        resolution: WorkflowStepResolution,
    ) -> bool {
        let report = if let Some(report) = self.build_check_report.as_ref().cloned() {
            report
        } else {
            if build_check_skip_enabled() {
                self.succeed_current_step(index, resolution);
                return true;
            }

            let repo_root = std::env::var_os(BUILD_CHECK_REPO_ROOT_ENV)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .or_else(|| self.build_check_repo_root.clone())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let mut request = build_check::BuildCheckRequest::for_repo_root(repo_root);
            if let Some(cargo_program) = self.build_check_cargo_program.as_ref() {
                request = request.with_cargo_program(cargo_program.clone());
            }
            request = request.apply_env_overrides();
            match build_check::run_build_check(&request) {
                Ok(report) => report,
                Err(err) => {
                    self.fail_current_step(
                        index,
                        resolution,
                        format!("build check executor failed: {err}"),
                    );
                    return true;
                }
            }
        };

        if report.success {
            self.succeed_current_step(index, resolution);
        } else {
            self.fail_current_step(index, resolution, Self::build_check_failure_reason(&report));
        }
        true
    }

    async fn run_tui_pty_gate_probe() -> TuiPtyGateResult {
        let path1 = std::path::Path::new("target/debug/vac");
        let path2 = std::path::Path::new("vac-rs/target/debug/vac");
        let binary_path = if path1.exists() {
            path1.to_path_buf()
        } else if path2.exists() {
            path2.to_path_buf()
        } else {
            std::path::PathBuf::from("vac")
        };

        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let envs = std::collections::HashMap::new();
        let program = binary_path.to_string_lossy().into_owned();
        let mut evidence = vec![
            format!("binary={program}"),
            format!("cwd={}", cwd.display()),
            "terminal_size=80x24".to_string(),
            "diagnostics=private/redacted; no environment dump captured".to_string(),
        ];

        let spawned = vac_utils_pty::pty::spawn_process(
            &program,
            &[],
            &cwd,
            &envs,
            &None,
            vac_utils_pty::TerminalSize { rows: 24, cols: 80 },
        )
        .await;

        let mut spawned = match spawned {
            Ok(spawned) => spawned,
            Err(err) => {
                return TuiPtyGateResult::blocked_operator(
                    format!("no interactive or virtual PTY available for root vac: {err}"),
                    evidence,
                );
            }
        };

        let mut initial_output_received = false;
        let read_timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1500));
        tokio::pin!(read_timeout);

        loop {
            tokio::select! {
                chunk = spawned.stdout_rx.recv() => {
                    if let Some(bytes) = chunk {
                        if !bytes.is_empty() {
                            initial_output_received = true;
                        }
                    } else {
                        break;
                    }
                }
                _ = &mut read_timeout => {
                    break;
                }
            }
        }

        if !initial_output_received {
            spawned.session.request_terminate();
            return TuiPtyGateResult::blocked_operator(
                "root vac produced no PTY screen output; slash-command visibility cannot be verified",
                evidence,
            );
        }
        evidence.push("root_vac_screen_output=observed".to_string());

        if let Err(err) = spawned.session.writer_sender().send(vec![b'/']).await {
            spawned.session.request_terminate();
            return TuiPtyGateResult::failed(
                format!("failed to send slash-command input into PTY: {err}"),
                None,
                evidence,
            );
        }

        let slash_timeout = tokio::time::sleep(tokio::time::Duration::from_millis(900));
        tokio::pin!(slash_timeout);
        let mut slash_output_received = false;
        loop {
            tokio::select! {
                chunk = spawned.stdout_rx.recv() => {
                    if let Some(bytes) = chunk {
                        if !bytes.is_empty() {
                            slash_output_received = true;
                        }
                    } else {
                        break;
                    }
                }
                _ = &mut slash_timeout => {
                    break;
                }
            }
        }

        if !slash_output_received {
            spawned.session.request_terminate();
            return TuiPtyGateResult::failed(
                "slash-command input produced no PTY screen update; slash-command list visibility cannot be verified",
                None,
                evidence,
            );
        }
        evidence.push("slash_input=screen_update_observed".to_string());

        if let Err(err) = spawned.session.writer_sender().send(vec![3u8]).await {
            spawned.session.request_terminate();
            return TuiPtyGateResult::failed(
                format!("failed to send Ctrl-C into PTY: {err}"),
                None,
                evidence,
            );
        }

        let exit_timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1200));
        tokio::pin!(exit_timeout);

        let exit_code = tokio::select! {
            code = spawned.exit_rx => code.ok(),
            _ = &mut exit_timeout => {
                spawned.session.request_terminate();
                None
            }
        };

        match exit_code {
            Some(0) => {
                evidence.push("ctrl_c_exit=exit_code_0".to_string());
                TuiPtyGateResult::passed(Some(0), evidence)
            }
            Some(code) => TuiPtyGateResult::failed(
                format!("PTY process exited with non-zero code {code}"),
                Some(code),
                evidence,
            ),
            None => TuiPtyGateResult::failed(
                "Ctrl-C exit could not be verified before timeout; terminal restoration is unproven",
                None,
                evidence,
            ),
        }
    }

    fn execute_tui_pty_gate(&mut self, index: usize, resolution: WorkflowStepResolution) -> bool {
        let result = if let Some(result) = self.tui_pty_gate_result.as_ref().cloned() {
            result
        } else {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(err) => {
                    self.fail_current_step(
                        index,
                        resolution,
                        format!("failed to initialize tokio runtime: {err}"),
                    );
                    return true;
                }
            };
            rt.block_on(Self::run_tui_pty_gate_probe())
        };

        if result.satisfies_release_gate() {
            self.succeed_current_step(index, resolution);
        } else {
            self.fail_current_step(index, resolution, result.report_reason());
        }

        true
    }

    fn execute_current_step_handler(
        &mut self,
        index: usize,
        resolution: WorkflowStepResolution,
    ) -> bool {
        if !resolution.supported() {
            let reason = resolution
                .blocked_reason
                .clone()
                .unwrap_or_else(|| unsupported_step_reason(&resolution.uses).to_string());
            self.fail_current_step(index, resolution, reason);
            return true;
        }

        match resolution.handler {
            Some(WorkflowStepHandler::RootSeedCoverage) => {
                match self.root_seed_registry_report.as_ref() {
                    Some(report) if report.is_failure() => {
                        let reason = report
                            .render_lines()
                            .iter()
                            .find(|line| line.contains("root seed coverage:"))
                            .cloned()
                            .unwrap_or_else(|| "root seed coverage failed".to_string());
                        self.fail_current_step(index, resolution, reason);
                    }
                    Some(_) => self.succeed_current_step(index, resolution),
                    None => self.fail_current_step(
                        index,
                        resolution,
                        "root seed coverage report unavailable".to_string(),
                    ),
                }
            }
            Some(WorkflowStepHandler::BuildCargoCheck) => {
                self.execute_build_cargo_check(index, resolution);
            }
            Some(WorkflowStepHandler::IdentityCheck) => match self.identity_check_report.as_ref() {
                Some(report) if !report.passed() => {
                    let reason = report
                        .failure_reason()
                        .unwrap_or_else(|| "identity check found forbidden terms".to_string());
                    self.fail_current_step(index, resolution, reason);
                }
                Some(_) => self.succeed_current_step(index, resolution),
                None => self.fail_current_step(
                    index,
                    resolution,
                    "identity check report unavailable".to_string(),
                ),
            },
            Some(WorkflowStepHandler::NoDuplicateTui) => {
                match self.no_duplicate_tui_report.as_ref() {
                    Some(report) if !report.passed() => {
                        let reason = report
                            .failure_reason()
                            .unwrap_or_else(|| "tui uniqueness found forbidden terms".to_string());
                        self.fail_current_step(index, resolution, reason);
                    }
                    Some(_) => self.succeed_current_step(index, resolution),
                    None => self.fail_current_step(
                        index,
                        resolution,
                        "tui uniqueness report unavailable".to_string(),
                    ),
                }
            }
            Some(WorkflowStepHandler::OwnershipScan) => match self.ownership_scan_report.as_ref() {
                Some(report) if report.ready_unowned_count() > 0 => {
                    let reason = format!(
                        "ownership scan found {} ready capability without ownership metadata",
                        report.ready_unowned_count()
                    );
                    self.fail_current_step(index, resolution, reason);
                }
                Some(_) => self.succeed_current_step(index, resolution),
                None => self.fail_current_step(
                    index,
                    resolution,
                    "ownership scan report unavailable".to_string(),
                ),
            },
            Some(WorkflowStepHandler::ArchitectureInvariants) => {
                match self.architecture_invariants_report.as_ref() {
                    Some(report) if report.is_failure() => {
                        let reason = report
                            .render_lines()
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "architecture invariants failed".to_string());
                        self.fail_current_step(index, resolution, reason);
                    }
                    Some(_) => self.succeed_current_step(index, resolution),
                    None => self.fail_current_step(
                        index,
                        resolution,
                        "architecture invariants report unavailable".to_string(),
                    ),
                }
            }
            Some(WorkflowStepHandler::TuiPtyGate) => {
                self.execute_tui_pty_gate(index, resolution);
            }
            Some(WorkflowStepHandler::ApprovalRequest) => {
                self.succeed_current_step(index, resolution);
            }
            Some(WorkflowStepHandler::RegistrySchemaCheck) => {
                match self.root_seed_registry_report.as_ref() {
                    Some(report) if report.is_failure() => {
                        let reason = report
                            .diagnostics()
                            .iter()
                            .map(|d| d.render_text())
                            .collect::<Vec<_>>()
                            .join("\n");
                        let reason = if reason.is_empty() {
                            "registry load failed".to_string()
                        } else {
                            reason
                        };
                        self.fail_current_step(index, resolution, reason);
                    }
                    Some(_) => self.succeed_current_step(index, resolution),
                    None => self.fail_current_step(
                        index,
                        resolution,
                        "registry report unavailable".to_string(),
                    ),
                }
            }
            Some(WorkflowStepHandler::CapabilityDashboardCheck) => {
                match self.ownership_scan_report.as_ref() {
                    Some(report) if report.ready_unowned_count() > 0 => {
                        let reason = format!(
                            "capability dashboard scan found {} ready capability without ownership metadata",
                            report.ready_unowned_count()
                        );
                        self.fail_current_step(index, resolution, reason);
                    }
                    Some(_) => self.succeed_current_step(index, resolution),
                    None => self.fail_current_step(
                        index,
                        resolution,
                        "ownership scan report unavailable".to_string(),
                    ),
                }
            }
            Some(WorkflowStepHandler::WorkflowBrowserCheck) => {
                match self.root_seed_registry_report.as_ref() {
                    Some(report) if report.is_failure() => {
                        let reason = report
                            .diagnostics()
                            .iter()
                            .map(|d| d.render_text())
                            .collect::<Vec<_>>()
                            .join("\n");
                        let reason = if reason.is_empty() {
                            "workflow registry load failed".to_string()
                        } else {
                            reason
                        };
                        self.fail_current_step(index, resolution, reason);
                    }
                    Some(_) => self.succeed_current_step(index, resolution),
                    None => self.fail_current_step(
                        index,
                        resolution,
                        "workflow report unavailable".to_string(),
                    ),
                }
            }
            Some(WorkflowStepHandler::ActivityEmit) => {
                self.succeed_current_step(index, resolution);
            }
            Some(WorkflowStepHandler::DonorMigrationInventoryCheck)
            | Some(WorkflowStepHandler::DonorMigrationDriftCheck)
            | Some(WorkflowStepHandler::DonorMigrationManifestCheck)
            | Some(WorkflowStepHandler::DonorMigrationReachabilityCheck)
            | Some(WorkflowStepHandler::DonorMigrationEvidenceCheck)
            | Some(WorkflowStepHandler::DonorMigrationCommitPhraseCheck) => {
                match self.donor_status_report.as_ref() {
                    Some(report) if report.is_failure() => {
                        let reason = format!(
                            "donor migration check failed ({} failures)",
                            report.failures.len()
                        );
                        self.fail_current_step(index, resolution, reason);
                    }
                    Some(_) => self.succeed_current_step(index, resolution),
                    None => self.fail_current_step(
                        index,
                        resolution,
                        "donor status report unavailable".to_string(),
                    ),
                }
            }
            None => {
                self.fail_current_step(index, resolution, "unsupported step use".to_string());
            }
        }
        true
    }

    fn succeed_current_step(&mut self, index: usize, resolution: WorkflowStepResolution) {
        self.completed_step_count += 1;
        self.current_step_lifecycle = Some(WorkflowStepLifecycle::Succeeded);
        self.state_trace.push(WorkflowExecutionState::Step {
            index,
            step_count: self.step_count,
            resolution: resolution.clone(),
            lifecycle: WorkflowStepLifecycle::Succeeded,
            started_step_count: self.started_step_count,
            completed_step_count: self.completed_step_count,
            waiting_approval_step_count: self.waiting_approval_step_count,
            blocked_step_count: self.blocked_step_count,
        });
        self.events
            .push(WorkflowExecutionEvent::StepSucceeded { index, resolution });
        self.cursor += 1;
        if self.cursor >= self.step_resolutions.len() {
            self.finished = true;
            self.state_trace.push(WorkflowExecutionState::Finished {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            });
            self.events.push(WorkflowExecutionEvent::Finished {
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            });
        }
    }

    pub fn expire_pending_approvals(
        &mut self,
        now: DateTime<Utc>,
    ) -> Result<usize, super::approval_lifecycle::ApprovalStoreError> {
        let due_ids = self
            .approval_requests
            .iter()
            .filter(|request| request.is_expired_at(now))
            .map(|request| request.id.clone())
            .collect::<Vec<_>>();
        let mut expired_count = 0;
        for id in due_ids {
            if self.resolve_approval(
                &id,
                ApprovalDecision::expired("approval request expired", now),
            )? {
                expired_count += 1;
            }
        }
        Ok(expired_count)
    }

    pub fn resolve_approval(
        &mut self,
        approval_request_id: &ApprovalRequestId,
        decision: ApprovalDecision,
    ) -> Result<bool, super::approval_lifecycle::ApprovalStoreError> {
        if self.cancelled_reason.is_some() || self.failed_reason.is_some() || self.finished {
            return Ok(false);
        }

        let persistent_decision = decision.clone();
        let resolved = self.approval_store.resolve(approval_request_id, decision)?;
        self.persist_resolved_approval_request(approval_request_id, persistent_decision)?;
        let reason = Self::approval_resolution_reason(&resolved);
        self.sync_resolved_approval_request(resolved.clone());
        self.events.push(WorkflowExecutionEvent::ApprovalResolved {
            approval_request_id: resolved.id.clone(),
            status: resolved.status,
            reason: reason.clone(),
        });

        match resolved.status {
            ApprovalStatus::Approved => {
                let Some(index) = self.current_step_index else {
                    return Ok(false);
                };
                let Some(resolution) = self.current_step_resolution.clone() else {
                    return Ok(false);
                };
                self.waiting_approval_step_count =
                    self.waiting_approval_step_count.saturating_sub(1);
                if let Some(reason) = self.policy_block_reason_for_step(index) {
                    self.fail_current_step(index, resolution, reason);
                    return Ok(true);
                }
                self.current_step_lifecycle = Some(WorkflowStepLifecycle::ResumedAfterApproval);
                self.state_trace.push(WorkflowExecutionState::Step {
                    index,
                    step_count: self.step_count,
                    resolution: resolution.clone(),
                    lifecycle: WorkflowStepLifecycle::ResumedAfterApproval,
                    started_step_count: self.started_step_count,
                    completed_step_count: self.completed_step_count,
                    waiting_approval_step_count: self.waiting_approval_step_count,
                    blocked_step_count: self.blocked_step_count,
                });
                self.execute_current_step_handler(index, resolution);
                Ok(true)
            }
            ApprovalStatus::Rejected | ApprovalStatus::Expired => {
                let index = self
                    .current_step_index
                    .unwrap_or(resolved.step_id.parse().unwrap_or(0));
                let resolution = self.current_step_resolution.clone().unwrap_or_else(|| {
                    WorkflowStepResolution {
                        id: resolved.step_id.clone(),
                        uses: resolved.capability_id.clone(),
                        handler: None,
                        blocked_reason: Some(reason.clone()),
                    }
                });
                self.waiting_approval_step_count =
                    self.waiting_approval_step_count.saturating_sub(1);
                self.blocked_step_count += 1;
                let terminal_reason = format!("approval {}: {}", resolved.status, reason);
                self.failed_reason = Some(terminal_reason.clone());
                self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                self.state_trace.push(WorkflowExecutionState::Step {
                    index,
                    step_count: self.step_count,
                    resolution: resolution.clone(),
                    lifecycle: WorkflowStepLifecycle::Failed,
                    started_step_count: self.started_step_count,
                    completed_step_count: self.completed_step_count,
                    waiting_approval_step_count: self.waiting_approval_step_count,
                    blocked_step_count: self.blocked_step_count,
                });
                self.events.push(WorkflowExecutionEvent::StepFailed {
                    index,
                    resolution,
                    reason: terminal_reason.clone(),
                });
                self.state_trace.push(WorkflowExecutionState::Failed {
                    workflow_id: self.workflow_id.clone(),
                    title: self.title.clone(),
                    reason: terminal_reason,
                    started_step_count: self.started_step_count,
                    completed_step_count: self.completed_step_count,
                    waiting_approval_step_count: self.waiting_approval_step_count,
                    blocked_step_count: self.blocked_step_count,
                });
                Ok(true)
            }
            ApprovalStatus::Pending => Ok(false),
        }
    }

    fn fail_current_step(
        &mut self,
        index: usize,
        resolution: WorkflowStepResolution,
        reason: String,
    ) {
        self.blocked_step_count += 1;
        self.failed_reason = Some(reason.clone());
        self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
        self.state_trace.push(WorkflowExecutionState::Step {
            index,
            step_count: self.step_count,
            resolution: resolution.clone(),
            lifecycle: WorkflowStepLifecycle::Failed,
            started_step_count: self.started_step_count,
            completed_step_count: self.completed_step_count,
            waiting_approval_step_count: self.waiting_approval_step_count,
            blocked_step_count: self.blocked_step_count,
        });
        self.events.push(WorkflowExecutionEvent::StepFailed {
            index,
            resolution,
            reason: reason.clone(),
        });
        self.state_trace.push(WorkflowExecutionState::Failed {
            workflow_id: self.workflow_id.clone(),
            title: self.title.clone(),
            reason,
            started_step_count: self.started_step_count,
            completed_step_count: self.completed_step_count,
            waiting_approval_step_count: self.waiting_approval_step_count,
            blocked_step_count: self.blocked_step_count,
        });
    }

    pub fn cancel(&mut self, reason: impl Into<String>) -> bool {
        if self.finished || self.failed_reason.is_some() || self.cancelled_reason.is_some() {
            return false;
        }

        let reason = reason.into();
        self.cancelled_reason = Some(reason.clone());
        self.state_trace.push(WorkflowExecutionState::Cancelled {
            workflow_id: self.workflow_id.clone(),
            title: self.title.clone(),
            reason: reason.clone(),
            started_step_count: self.started_step_count,
            completed_step_count: self.completed_step_count,
            waiting_approval_step_count: self.waiting_approval_step_count,
            blocked_step_count: self.blocked_step_count,
        });
        self.events
            .push(WorkflowExecutionEvent::Cancelled { reason });
        true
    }

    pub fn advance(&mut self) -> bool {
        if self.cancelled_reason.is_some() || self.failed_reason.is_some() || self.finished {
            return false;
        }

        if !self.started {
            self.started = true;
            self.events.push(WorkflowExecutionEvent::Started {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                step_count: self.step_count,
            });
            return true;
        }

        if self.waiting_approval_step_count > 0 {
            return false;
        }

        let Some(resolution) = self.step_resolutions.get(self.cursor).cloned() else {
            self.finished = true;
            self.state_trace.push(WorkflowExecutionState::Finished {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            });
            self.events.push(WorkflowExecutionEvent::Finished {
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            });
            return true;
        };

        let index = self.cursor + 1;
        self.current_step_index = Some(index);
        self.current_step_resolution = Some(resolution.clone());
        self.current_step_lifecycle = Some(WorkflowStepLifecycle::Running);
        self.started_step_count += 1;
        self.state_trace.push(WorkflowExecutionState::Step {
            index,
            step_count: self.step_count,
            resolution: resolution.clone(),
            lifecycle: WorkflowStepLifecycle::Running,
            started_step_count: self.started_step_count,
            completed_step_count: self.completed_step_count,
            waiting_approval_step_count: self.waiting_approval_step_count,
            blocked_step_count: self.blocked_step_count,
        });
        self.events.push(WorkflowExecutionEvent::StepStarted {
            index,
            resolution: resolution.clone(),
        });

        if let Some(reason) = self.policy_block_reason_for_step(index) {
            self.fail_current_step(index, resolution, reason);
            return true;
        }

        if let Some(approval_check) = self.approval_checks.get(index.saturating_sub(1)) {
            match approval_check.decision {
                WorkflowApprovalDecision::Blocked => {
                    let reason = approval_check
                        .reason
                        .clone()
                        .unwrap_or_else(|| "approval policy blocked workflow step".to_string());
                    self.blocked_step_count += 1;
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "approval policy blocked workflow step".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
                WorkflowApprovalDecision::ApprovalRequired => {
                    self.waiting_approval_step_count += 1;
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::WaitingApproval);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::WaitingApproval,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    let approval_request_id = match self.request_approval_for_step(
                        &resolution,
                        approval_check
                            .reason
                            .clone()
                            .map(|reason| vec![reason])
                            .unwrap_or_else(|| {
                                vec!["approval policy requires operator approval".to_string()]
                            }),
                    ) {
                        Ok(approval_request_id) => approval_request_id,
                        Err(err) => {
                            self.fail_current_step(
                                index,
                                resolution.clone(),
                                format!("approval persistence failed: {err}"),
                            );
                            return false;
                        }
                    };
                    self.events
                        .push(WorkflowExecutionEvent::StepWaitingApproval {
                            index,
                            resolution,
                            approval_request_id,
                        });
                    return true;
                }
                WorkflowApprovalDecision::Allowed => {}
            }
        }

        if let Some(Some(decision)) = self.policy_decisions.get(index.saturating_sub(1)) {
            use super::policy_manifest::PolicyDecisionReport;
            match decision.clone() {
                PolicyDecisionReport::Block { reasons } => {
                    self.blocked_step_count += 1;
                    let reason = format!("policy: {}", reasons.join("; "));
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "policy block".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
                PolicyDecisionReport::RequireApproval { reasons }
                | PolicyDecisionReport::UnknownPolicy { reasons } => {
                    self.waiting_approval_step_count += 1;
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::WaitingApproval);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::WaitingApproval,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    let approval_request_id =
                        match self.request_approval_for_step(&resolution, reasons) {
                            Ok(approval_request_id) => approval_request_id,
                            Err(err) => {
                                self.fail_current_step(
                                    index,
                                    resolution.clone(),
                                    format!("approval persistence failed: {err}"),
                                );
                                return false;
                            }
                        };
                    self.events
                        .push(WorkflowExecutionEvent::StepWaitingApproval {
                            index,
                            resolution,
                            approval_request_id,
                        });
                    return true;
                }
                PolicyDecisionReport::Allow { .. } => {}
            }
        }

        if !resolution.supported() {
            let reason = resolution
                .blocked_reason
                .clone()
                .unwrap_or_else(|| unsupported_step_reason(&resolution.uses).to_string());
            self.blocked_step_count += 1;
            self.failed_reason = Some(reason.clone());
            self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
            self.state_trace.push(WorkflowExecutionState::Step {
                index,
                step_count: self.step_count,
                resolution: resolution.clone(),
                lifecycle: WorkflowStepLifecycle::Failed,
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            });
            self.events.push(WorkflowExecutionEvent::StepFailed {
                index,
                resolution,
                reason,
            });
            self.state_trace.push(WorkflowExecutionState::Failed {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                reason: self
                    .failed_reason
                    .clone()
                    .unwrap_or_else(|| "unsupported step use".to_string()),
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            });
            return true;
        }

        if matches!(
            resolution.handler,
            Some(WorkflowStepHandler::RootSeedCoverage)
        ) {
            match self.root_seed_registry_report.as_ref() {
                Some(report) if report.is_failure() => {
                    let reason = report
                        .render_lines()
                        .iter()
                        .find(|line| line.contains("root seed coverage:"))
                        .cloned()
                        .unwrap_or_else(|| "root seed coverage failed".to_string());
                    self.blocked_step_count += 1;
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "root seed coverage failed".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
                Some(_) => {}
                None => {
                    let reason = "root seed coverage report unavailable".to_string();
                    self.blocked_step_count += 1;
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "root seed coverage failed".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
            }
        }

        if matches!(resolution.handler, Some(WorkflowStepHandler::IdentityCheck)) {
            match self.identity_check_report.as_ref() {
                Some(report) if !report.passed() => {
                    let reason = report
                        .failure_reason()
                        .unwrap_or_else(|| "identity check found forbidden terms".to_string());
                    self.fail_current_step(index, resolution, reason);
                    return true;
                }
                Some(_) => {}
                None => {
                    self.fail_current_step(
                        index,
                        resolution,
                        "identity check report unavailable".to_string(),
                    );
                    return true;
                }
            }
        }

        if matches!(
            resolution.handler,
            Some(WorkflowStepHandler::NoDuplicateTui)
        ) {
            match self.no_duplicate_tui_report.as_ref() {
                Some(report) if !report.passed() => {
                    let reason = report
                        .failure_reason()
                        .unwrap_or_else(|| "tui uniqueness found forbidden terms".to_string());
                    self.blocked_step_count += 1;
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "tui uniqueness failed".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
                Some(_) => {}
                None => {
                    let reason = "tui uniqueness report unavailable".to_string();
                    self.blocked_step_count += 1;
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "tui uniqueness failed".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
            }
        }

        if matches!(
            resolution.handler,
            Some(WorkflowStepHandler::ArchitectureInvariants)
        ) {
            match self.architecture_invariants_report.as_ref() {
                Some(report) if report.is_failure() => {
                    let reason = report
                        .render_lines()
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "architecture invariants failed".to_string());
                    self.blocked_step_count += 1;
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "architecture invariants failed".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
                Some(_) => {}
                None => {
                    let reason = "architecture invariants report unavailable".to_string();
                    self.blocked_step_count += 1;
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "architecture invariants failed".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
            }
        }

        if matches!(resolution.handler, Some(WorkflowStepHandler::OwnershipScan)) {
            match self.ownership_scan_report.as_ref() {
                Some(report) if report.ready_unowned_count() > 0 => {
                    let reason = format!(
                        "ownership scan found {} ready capability without ownership metadata",
                        report.ready_unowned_count()
                    );
                    self.blocked_step_count += 1;
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "ownership scan failed".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
                Some(_) => {}
                None => {
                    let reason = "ownership scan report unavailable".to_string();
                    self.blocked_step_count += 1;
                    self.failed_reason = Some(reason.clone());
                    self.current_step_lifecycle = Some(WorkflowStepLifecycle::Failed);
                    self.state_trace.push(WorkflowExecutionState::Step {
                        index,
                        step_count: self.step_count,
                        resolution: resolution.clone(),
                        lifecycle: WorkflowStepLifecycle::Failed,
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    self.events.push(WorkflowExecutionEvent::StepFailed {
                        index,
                        resolution,
                        reason,
                    });
                    self.state_trace.push(WorkflowExecutionState::Failed {
                        workflow_id: self.workflow_id.clone(),
                        title: self.title.clone(),
                        reason: self
                            .failed_reason
                            .clone()
                            .unwrap_or_else(|| "ownership scan failed".to_string()),
                        started_step_count: self.started_step_count,
                        completed_step_count: self.completed_step_count,
                        waiting_approval_step_count: self.waiting_approval_step_count,
                        blocked_step_count: self.blocked_step_count,
                    });
                    return true;
                }
            }
        }

        if matches!(
            resolution.handler,
            Some(WorkflowStepHandler::RegistrySchemaCheck)
        ) {
            match self.root_seed_registry_report.as_ref() {
                Some(report) if report.is_failure() => {
                    let reason = report
                        .diagnostics()
                        .iter()
                        .map(|d| d.render_text())
                        .collect::<Vec<_>>()
                        .join("\n");
                    let reason = if reason.is_empty() {
                        "registry load failed".to_string()
                    } else {
                        reason
                    };
                    self.fail_current_step(index, resolution, reason);
                    return true;
                }
                Some(_) => {}
                None => {
                    self.fail_current_step(
                        index,
                        resolution,
                        "registry report unavailable".to_string(),
                    );
                    return true;
                }
            }
        }

        if matches!(
            resolution.handler,
            Some(WorkflowStepHandler::CapabilityDashboardCheck)
        ) {
            match self.ownership_scan_report.as_ref() {
                Some(report) if report.ready_unowned_count() > 0 => {
                    let reason = format!(
                        "capability dashboard scan found {} ready capability without ownership metadata",
                        report.ready_unowned_count()
                    );
                    self.fail_current_step(index, resolution, reason);
                    return true;
                }
                Some(_) => {}
                None => {
                    self.fail_current_step(
                        index,
                        resolution,
                        "ownership scan report unavailable".to_string(),
                    );
                    return true;
                }
            }
        }

        if matches!(
            resolution.handler,
            Some(WorkflowStepHandler::WorkflowBrowserCheck)
        ) {
            match self.root_seed_registry_report.as_ref() {
                Some(report) if report.is_failure() => {
                    let reason = report
                        .diagnostics()
                        .iter()
                        .map(|d| d.render_text())
                        .collect::<Vec<_>>()
                        .join("\n");
                    let reason = if reason.is_empty() {
                        "workflow registry load failed".to_string()
                    } else {
                        reason
                    };
                    self.fail_current_step(index, resolution, reason);
                    return true;
                }
                Some(_) => {}
                None => {
                    self.fail_current_step(
                        index,
                        resolution,
                        "workflow report unavailable".to_string(),
                    );
                    return true;
                }
            }
        }

        if matches!(
            resolution.handler,
            Some(WorkflowStepHandler::BuildCargoCheck)
        ) {
            self.execute_build_cargo_check(index, resolution);
            return true;
        }

        if matches!(resolution.handler, Some(WorkflowStepHandler::TuiPtyGate)) {
            self.execute_tui_pty_gate(index, resolution);
            return true;
        }

        if matches!(
            resolution.handler,
            Some(WorkflowStepHandler::ApprovalRequest)
        ) {
            self.waiting_approval_step_count += 1;
            self.current_step_lifecycle = Some(WorkflowStepLifecycle::WaitingApproval);
            self.state_trace.push(WorkflowExecutionState::Step {
                index,
                step_count: self.step_count,
                resolution: resolution.clone(),
                lifecycle: WorkflowStepLifecycle::WaitingApproval,
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            });
            let approval_request_id = match self.request_approval_for_step(
                &resolution,
                vec!["workflow step requests explicit operator approval".to_string()],
            ) {
                Ok(approval_request_id) => approval_request_id,
                Err(err) => {
                    self.fail_current_step(
                        index,
                        resolution.clone(),
                        format!("approval persistence failed: {err}"),
                    );
                    return false;
                }
            };
            self.events
                .push(WorkflowExecutionEvent::StepWaitingApproval {
                    index,
                    resolution,
                    approval_request_id,
                });
            return true;
        }

        if matches!(
            resolution.handler,
            Some(WorkflowStepHandler::DonorMigrationInventoryCheck)
                | Some(WorkflowStepHandler::DonorMigrationDriftCheck)
                | Some(WorkflowStepHandler::DonorMigrationManifestCheck)
                | Some(WorkflowStepHandler::DonorMigrationReachabilityCheck)
                | Some(WorkflowStepHandler::DonorMigrationEvidenceCheck)
                | Some(WorkflowStepHandler::DonorMigrationCommitPhraseCheck)
        ) {
            match self.donor_status_report.as_ref() {
                Some(report) if report.is_failure() => {
                    let reason = format!(
                        "donor migration check failed ({} failures)",
                        report.failures.len()
                    );
                    self.fail_current_step(index, resolution, reason);
                    return true;
                }
                Some(_) => {
                    self.succeed_current_step(index, resolution);
                    return true;
                }
                None => {
                    self.fail_current_step(
                        index,
                        resolution,
                        "donor status report unavailable".to_string(),
                    );
                    return true;
                }
            }
        }

        if let Some(reason) = Self::unimplemented_handler_reason(&resolution) {
            self.fail_current_step(index, resolution, reason);
            return true;
        }

        self.completed_step_count += 1;
        self.current_step_lifecycle = Some(WorkflowStepLifecycle::Succeeded);
        self.state_trace.push(WorkflowExecutionState::Step {
            index,
            step_count: self.step_count,
            resolution: resolution.clone(),
            lifecycle: WorkflowStepLifecycle::Succeeded,
            started_step_count: self.started_step_count,
            completed_step_count: self.completed_step_count,
            waiting_approval_step_count: self.waiting_approval_step_count,
            blocked_step_count: self.blocked_step_count,
        });
        self.events
            .push(WorkflowExecutionEvent::StepSucceeded { index, resolution });
        self.cursor += 1;
        if self.cursor >= self.step_resolutions.len() {
            self.finished = true;
            self.state_trace.push(WorkflowExecutionState::Finished {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            });
            self.events.push(WorkflowExecutionEvent::Finished {
                started_step_count: self.started_step_count,
                completed_step_count: self.completed_step_count,
                waiting_approval_step_count: self.waiting_approval_step_count,
                blocked_step_count: self.blocked_step_count,
            });
        }
        true
    }

    pub fn run(mut self) -> WorkflowExecutionReport {
        while self.advance() {}
        WorkflowExecutionReport {
            state_trace: self.state_trace,
            events: self.events,
            approval_requests: self.approval_requests,
        }
    }
}

