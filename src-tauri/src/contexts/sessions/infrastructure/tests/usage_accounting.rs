use super::*;

fn seed_accounting_scope(fixture: &Fixture) {
    fixture
        .database
        .connection()
        .expect("accounting connection")
        .execute_batch(
            r#"
            INSERT INTO agents (id, display_name, provider, launch_kind)
            VALUES ('accounting-agent', 'Accounting Agent', 'test', 'api');
            INSERT INTO sessions
                (id, title, agent_id, interaction_mode, lifecycle_state, created_at, updated_at)
            VALUES
                ('accounting-session', 'Accounting Session', 'accounting-agent', 'api', 'idle',
                 '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z');
            "#,
        )
        .expect("seed accounting scope");
}

fn accounting_invocation(id: &str) -> NewModelInvocation {
    NewModelInvocation {
        id: id.to_string(),
        generation_id: Some("generation-1".to_string()),
        run_id: Some("run-1".to_string()),
        operation_id: Some("operation-1".to_string()),
        session_id: "accounting-session".to_string(),
        message_id: None,
        agent_id: "accounting-agent".to_string(),
        provider_id: Some("provider-1".to_string()),
        profile_id: Some("profile-1".to_string()),
        endpoint_id: Some("endpoint-1".to_string()),
        model_id: Some("model-1".to_string()),
        interaction_kind: UsageInteractionKind::NativeApi,
        purpose: UsagePurpose::AssistantInitial,
        request_sequence: 0,
        attempt: 0,
        started_at: "2026-08-12T00:00:01Z".to_string(),
    }
}

fn accounting_observation(
    id: &str,
    source_key: &str,
    quality: MeasurementQuality,
    output: i64,
) -> NewUsageObservation {
    NewUsageObservation {
        id: id.to_string(),
        invocation_id: "invocation-1".to_string(),
        quality,
        unit: if quality == MeasurementQuality::Estimated {
            AccountingUnit::Characters
        } else {
            AccountingUnit::Tokens
        },
        measurement_kind: MeasurementKind::Interval,
        dimensions: TokenDimensions {
            input: 10,
            output,
            provider_total: (quality != MeasurementQuality::Estimated).then_some(10 + output),
            ..TokenDimensions::default()
        },
        cache_overlap: TokenOverlap::Subset,
        reasoning_overlap: TokenOverlap::Subset,
        normalization_version: "test-v1".to_string(),
        source: "test-provider".to_string(),
        source_key: source_key.to_string(),
        source_revision: Some("1".to_string()),
        supersedes_observation_id: None,
        event_at: Some("2026-08-12T00:00:02Z".to_string()),
        observed_at: "2026-08-12T00:00:03Z".to_string(),
        provenance_hash: Some("safe-hash".to_string()),
    }
}

#[test]
fn accounting_ledger_is_idempotent_and_supersedes_estimates() {
    let fixture = fixture("accounting-ledger-idempotency");
    seed_accounting_scope(&fixture);
    let invocation = fixture
        .repository
        .start_invocation(&accounting_invocation("invocation-1"))
        .expect("start invocation");
    assert_eq!(invocation.status, UsageStatus::Running);
    let mut conflicting_invocation = accounting_invocation("invocation-1");
    conflicting_invocation.model_id = Some("different-model".to_string());
    assert!(matches!(
        fixture.repository.start_invocation(&conflicting_invocation),
        Err(SessionsApplicationError::Validation(_))
    ));

    let estimated = accounting_observation(
        "observation-estimated",
        "test:request-1:estimated",
        MeasurementQuality::Estimated,
        20,
    );
    let first = fixture
        .repository
        .record_observation(&estimated)
        .expect("record estimate");
    let mut replay = estimated.clone();
    replay.id = "ignored-replay-id".to_string();
    replay.observed_at = "2026-08-12T00:01:00Z".to_string();
    let replayed = fixture
        .repository
        .record_observation(&replay)
        .expect("replay observation");
    assert_eq!(replayed.observation.id, first.observation.id);

    let mut mismatch = replay.clone();
    mismatch.dimensions.output = 21;
    assert!(matches!(
        fixture.repository.record_observation(&mismatch),
        Err(SessionsApplicationError::Validation(_))
    ));

    let mut reported = accounting_observation(
        "observation-reported",
        "test:request-1:reported",
        MeasurementQuality::Reported,
        7,
    );
    reported.supersedes_observation_id = Some(first.observation.id.clone());
    fixture
        .repository
        .record_observation(&reported)
        .expect("upgrade estimate");
    fixture
        .repository
        .finalize_invocation(
            "invocation-1",
            UsageStatus::Succeeded,
            "2026-08-12T00:00:04Z",
        )
        .expect("finalize invocation");
    assert!(matches!(
        fixture.repository.finalize_invocation(
            "invocation-1",
            UsageStatus::Failed,
            "2026-08-12T00:00:05Z",
        ),
        Err(SessionsApplicationError::Validation(_))
    ));

    let details = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: Some(MeasurementQuality::Reported),
            status: Some(UsageStatus::Succeeded),
            after_id: None,
            limit: 10,
        })
        .expect("query details");
    assert_eq!(details.invocations.len(), 1);
    assert_eq!(details.observations.len(), 1);
    assert_eq!(details.observations[0].observation.dimensions.output, 7);

    let connection = fixture.database.connection().expect("ledger connection");
    let active: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM token_usage_observations WHERE superseded_by_observation_id IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("active observations");
    assert_eq!(active, 1);
}

#[test]
fn cumulative_cursor_rejects_stale_advances_and_requires_reset_epochs() {
    let fixture = fixture("accounting-cursor-cas");
    let first = UsageCursor {
        source_id: "codex:session-1".to_string(),
        provider_session_id: "provider-session-1".to_string(),
        epoch: 0,
        dimensions: TokenDimensions {
            input: 10,
            output: 5,
            provider_total: Some(15),
            ..TokenDimensions::default()
        },
        ordering_key: "001".to_string(),
        source_revision: Some("1".to_string()),
        revision: 0,
        updated_at: "2026-08-12T00:00:01Z".to_string(),
    };
    fixture
        .repository
        .advance_cursor(&UsageCursorAdvance {
            previous: None,
            current: first.clone(),
            observation: None,
        })
        .expect("open cursor");

    let mut second = first.clone();
    second.dimensions.input = 14;
    second.dimensions.provider_total = Some(19);
    second.ordering_key = "002".to_string();
    second.revision = 1;
    fixture
        .repository
        .advance_cursor(&UsageCursorAdvance {
            previous: Some(first.clone()),
            current: second.clone(),
            observation: None,
        })
        .expect("advance cursor");

    assert!(matches!(
        fixture.repository.advance_cursor(&UsageCursorAdvance {
            previous: Some(first),
            current: second.clone(),
            observation: None,
        }),
        Err(SessionsApplicationError::Transaction(_))
    ));

    let mut invalid_reset = second.clone();
    invalid_reset.dimensions = TokenDimensions::default();
    invalid_reset.ordering_key = "003".to_string();
    invalid_reset.revision = 2;
    assert!(matches!(
        fixture.repository.advance_cursor(&UsageCursorAdvance {
            previous: Some(second.clone()),
            current: invalid_reset.clone(),
            observation: None,
        }),
        Err(SessionsApplicationError::Validation(_))
    ));

    invalid_reset.epoch = 1;
    let reset = fixture
        .repository
        .advance_cursor(&UsageCursorAdvance {
            previous: Some(second),
            current: invalid_reset,
            observation: None,
        })
        .expect("open reset epoch");
    assert_eq!(reset.epoch, 1);
}

#[test]
fn ledger_projection_separates_quality_purpose_and_failed_usage() {
    let fixture = fixture("accounting-ledger-projection");
    seed_accounting_scope(&fixture);
    let cases = [
        (
            "invocation-1",
            UsagePurpose::AssistantInitial,
            UsageStatus::Succeeded,
            MeasurementQuality::Reported,
            AccountingUnit::Tokens,
            TokenDimensions {
                input: 10,
                output: 7,
                provider_total: Some(17),
                ..TokenDimensions::default()
            },
            TokenOverlap::Unknown,
        ),
        (
            "invocation-2",
            UsagePurpose::ContextCompaction,
            UsageStatus::Succeeded,
            MeasurementQuality::ReportedDerived,
            AccountingUnit::Tokens,
            TokenDimensions {
                input: 5,
                output: 2,
                ..TokenDimensions::default()
            },
            TokenOverlap::Subset,
        ),
        (
            "invocation-3",
            UsagePurpose::ToolContinuation,
            UsageStatus::Failed,
            MeasurementQuality::Estimated,
            AccountingUnit::Characters,
            TokenDimensions {
                input: 100,
                output: 20,
                ..TokenDimensions::default()
            },
            TokenOverlap::Subset,
        ),
        (
            "invocation-4",
            UsagePurpose::TerminalInterval,
            UsageStatus::Cancelled,
            MeasurementQuality::Reported,
            AccountingUnit::Tokens,
            TokenDimensions {
                cached_input: 8,
                ..TokenDimensions::default()
            },
            TokenOverlap::Exclusive,
        ),
    ];
    for (index, (id, purpose, status, quality, unit, dimensions, cache_overlap)) in
        cases.into_iter().enumerate()
    {
        let mut invocation = accounting_invocation(id);
        invocation.purpose = purpose;
        invocation.request_sequence = u32::try_from(index).expect("bounded fixture index");
        invocation.provider_id = (id != "invocation-4").then(|| "provider-1".to_string());
        fixture
            .repository
            .start_invocation(&invocation)
            .expect("start projected invocation");
        let mut observation = accounting_observation(
            &format!("observation-{id}"),
            &format!("test:{id}"),
            quality,
            dimensions.output,
        );
        observation.invocation_id = id.to_string();
        observation.unit = unit;
        observation.dimensions = dimensions;
        observation.cache_overlap = cache_overlap;
        observation.reasoning_overlap = TokenOverlap::Subset;
        fixture
            .repository
            .record_observation(&observation)
            .expect("record projected observation");
        fixture
            .repository
            .finalize_invocation(id, status, "2026-08-12T00:00:04Z")
            .expect("finalize projected invocation");
    }

    let summary = fixture
        .repository
        .usage_summary(&UsageSummaryQuery {
            session_id: Some("accounting-session".to_string()),
            message_id: None,
            generation_id: None,
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            range_start: Some("2026-08-12T00:00:00Z".to_string()),
            range_end: Some("2026-08-13T00:00:00Z".to_string()),
            breakdown_limit: 10,
            generated_at: "2026-08-12T01:00:00Z".to_string(),
        })
        .expect("project usage summary");
    assert_eq!(summary.counts.calls, 4);
    assert_eq!(summary.counts.generations, 1);
    assert_eq!(summary.counts.sessions, 1);
    assert_eq!(summary.totals.reported.headline_total, Some(25));
    assert_eq!(summary.totals.reported.dimensions.cached_input, 8);
    assert_eq!(summary.totals.reported_derived.headline_total, Some(7));
    assert_eq!(summary.totals.estimated.headline_total, Some(120));
    assert_eq!(summary.internal.reported_derived.headline_total, Some(7));
    assert_eq!(summary.user_response.reported.headline_total, Some(25));
    assert_eq!(summary.daily.len(), 1);
    let provider_breakdown = summary
        .breakdowns
        .iter()
        .find(|breakdown| breakdown.dimension == UsageBreakdownDimension::Provider)
        .expect("provider breakdown");
    assert_eq!(provider_breakdown.entries[0].key, "provider-1");
    assert_eq!(provider_breakdown.entries[0].counts.calls, 3);
    assert_eq!(provider_breakdown.entries[1].key, "unknown");

    let failed = fixture
        .repository
        .usage_summary(&UsageSummaryQuery {
            session_id: None,
            message_id: None,
            generation_id: None,
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: Some(UsageStatus::Failed),
            range_start: None,
            range_end: None,
            breakdown_limit: 1,
            generated_at: "2026-08-12T01:00:00Z".to_string(),
        })
        .expect("filter failed usage");
    assert_eq!(failed.counts.calls, 1);
    assert_eq!(failed.totals.estimated.headline_total, Some(120));
    assert_eq!(failed.totals.reported.headline_total, Some(0));

    let page = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: None,
            limit: 1,
        })
        .expect("first invocation page");
    assert_eq!(page.invocations.len(), 1);
    assert_eq!(page.next_cursor.as_deref(), Some("invocation-1"));
}

#[test]
fn ledger_projection_preserves_authoritative_cache_reasoning_and_unknown_semantics() {
    let fixture = fixture("accounting-ledger-semantics");
    seed_accounting_scope(&fixture);
    let cases = [
        (
            "invocation-authoritative",
            "model-authoritative",
            TokenDimensions {
                input: 10,
                output: 5,
                cached_input: 100,
                reasoning_output: 50,
                provider_total: Some(20),
                ..TokenDimensions::default()
            },
            TokenOverlap::Unknown,
            TokenOverlap::Unknown,
            Some(20),
        ),
        (
            "invocation-cache-only",
            "model-cache-only",
            TokenDimensions {
                cached_input: 7,
                ..TokenDimensions::default()
            },
            TokenOverlap::Exclusive,
            TokenOverlap::Subset,
            Some(7),
        ),
        (
            "invocation-reasoning",
            "model-reasoning",
            TokenDimensions {
                reasoning_output: 9,
                ..TokenDimensions::default()
            },
            TokenOverlap::Subset,
            TokenOverlap::Exclusive,
            Some(9),
        ),
        (
            "invocation-unknown",
            "model-unknown",
            TokenDimensions {
                input: 3,
                cached_input: 2,
                ..TokenDimensions::default()
            },
            TokenOverlap::Unknown,
            TokenOverlap::Subset,
            None,
        ),
    ];
    for (sequence, (id, model, dimensions, cache_overlap, reasoning_overlap, expected)) in
        cases.into_iter().enumerate()
    {
        let mut invocation = accounting_invocation(id);
        invocation.model_id = Some(model.to_string());
        invocation.request_sequence = u32::try_from(sequence).expect("bounded sequence");
        fixture
            .repository
            .start_invocation(&invocation)
            .expect("start semantic invocation");
        let mut observation = accounting_observation(
            &format!("observation-{id}"),
            &format!("semantic:{id}"),
            MeasurementQuality::Reported,
            dimensions.output,
        );
        observation.invocation_id = id.to_string();
        observation.dimensions = dimensions;
        observation.cache_overlap = cache_overlap;
        observation.reasoning_overlap = reasoning_overlap;
        fixture
            .repository
            .record_observation(&observation)
            .expect("record semantic observation");
        fixture
            .repository
            .finalize_invocation(id, UsageStatus::Succeeded, "2026-08-12T00:00:04Z")
            .expect("finalize semantic invocation");

        let summary = fixture
            .repository
            .usage_summary(&UsageSummaryQuery {
                session_id: Some("accounting-session".to_string()),
                message_id: None,
                generation_id: None,
                agent_id: None,
                provider_id: None,
                model_id: Some(model.to_string()),
                purpose: Some(UsagePurpose::AssistantInitial),
                quality: Some(MeasurementQuality::Reported),
                status: Some(UsageStatus::Succeeded),
                range_start: None,
                range_end: None,
                breakdown_limit: 10,
                generated_at: "2026-08-12T01:00:00Z".to_string(),
            })
            .expect("query semantic summary");
        assert_eq!(summary.counts.calls, 1);
        assert_eq!(summary.totals.reported.headline_total, expected);
        assert_eq!(summary.totals.reported.dimensions, dimensions);
    }

    let first = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: None,
            limit: 2,
        })
        .expect("first bounded details page");
    assert_eq!(first.invocations.len(), 2);
    let cursor = first.next_cursor.expect("first page cursor");
    let second = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: Some(cursor),
            limit: 2,
        })
        .expect("second bounded details page");
    assert_eq!(second.invocations.len(), 2);
    assert!(second.next_cursor.is_none());

    let empty = fixture
        .repository
        .usage_summary(&UsageSummaryQuery {
            session_id: Some("accounting-session".to_string()),
            message_id: None,
            generation_id: None,
            agent_id: None,
            provider_id: Some("missing-provider".to_string()),
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            range_start: None,
            range_end: None,
            breakdown_limit: 10,
            generated_at: "2026-08-12T01:00:00Z".to_string(),
        })
        .expect("empty filtered summary");
    assert_eq!(empty.counts.calls, 0);
    assert!(empty.daily.is_empty());
    assert_eq!(empty.totals.reported.headline_total, Some(0));
}

#[test]
fn accounting_schema_cascades_and_never_projects_message_content() {
    let fixture = fixture("accounting-ledger-integrity");
    seed_accounting_scope(&fixture);
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO messages (id, session_id, role, status, content, created_at, updated_at)
             VALUES ('secret-message', 'accounting-session', 'user', 'completed', ?1, ?2, ?2)",
            params!["prompt-secret-never-project", "2026-08-12T00:00:00Z"],
        )
        .expect("seed private message content");
    drop(connection);

    let mut invocation = accounting_invocation("invocation-1");
    invocation.message_id = Some("secret-message".to_string());
    fixture
        .repository
        .start_invocation(&invocation)
        .expect("start invocation");
    fixture
        .repository
        .record_observation(&accounting_observation(
            "observation-1",
            "safe:source:1",
            MeasurementQuality::Reported,
            4,
        ))
        .expect("record observation");
    fixture
        .repository
        .finalize_invocation("invocation-1", UsageStatus::Failed, "2026-08-12T00:00:04Z")
        .expect("finalize failed invocation");

    let details = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: None,
            limit: 10,
        })
        .expect("safe details");
    let rendered = format!("{details:?}");
    assert!(!rendered.contains("prompt-secret-never-project"));
    for forbidden in [
        "prompt",
        "response",
        "credential",
        "header",
        "tool_payload",
        "raw_frame",
    ] {
        let count: i64 = fixture
            .database
            .connection()
            .expect("schema connection")
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('token_usage_observations')
                 WHERE lower(name) LIKE '%' || ?1 || '%'",
                [forbidden],
                |row| row.get(0),
            )
            .expect("inspect accounting columns");
        assert_eq!(count, 0, "forbidden accounting column: {forbidden}");
    }

    SessionTransactionPort::delete_session(
        &fixture.repository,
        &SessionId::parse("accounting-session").expect("session id"),
    )
    .expect("delete accounting session");
    let connection = fixture.database.connection().expect("cascade connection");
    for table in ["model_invocations", "token_usage_observations"] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("cascade count");
        assert_eq!(count, 0, "{table} must cascade with its session");
    }
    let mut unknown = accounting_invocation("unknown-invocation");
    unknown.session_id = "missing-session".to_string();
    assert!(fixture.repository.start_invocation(&unknown).is_err());
}

#[test]
fn accounting_queries_are_bounded_under_cardinality_and_concurrent_ingestion() {
    let fixture = fixture("accounting-ledger-bounded-concurrency");
    seed_accounting_scope(&fixture);
    for index in 0..50_u32 {
        let id = format!("bulk-invocation-{index:03}");
        let mut invocation = accounting_invocation(&id);
        invocation.generation_id = Some(format!("bulk-generation-{index:03}"));
        invocation.provider_id = Some(format!("provider-{index:03}"));
        invocation.model_id = Some(format!("model-{index:03}"));
        invocation.request_sequence = index;
        fixture
            .repository
            .start_invocation(&invocation)
            .expect("start bulk invocation");
        let mut observation = accounting_observation(
            &format!("bulk-observation-{index:03}"),
            &format!("bulk:source:{index:03}"),
            MeasurementQuality::Reported,
            1,
        );
        observation.invocation_id = id.clone();
        observation.event_at = Some("2026-08-10T12:00:00Z".to_string());
        fixture
            .repository
            .record_observation(&observation)
            .expect("record bulk observation");
        fixture
            .repository
            .finalize_invocation(&id, UsageStatus::Succeeded, "2026-08-12T00:00:04Z")
            .expect("finalize bulk invocation");
    }
    let query = UsageSummaryQuery {
        session_id: Some("accounting-session".to_string()),
        message_id: None,
        generation_id: None,
        agent_id: None,
        provider_id: None,
        model_id: None,
        purpose: None,
        quality: None,
        status: None,
        range_start: None,
        range_end: None,
        breakdown_limit: 5,
        generated_at: "2026-08-12T01:00:00Z".to_string(),
    };
    let summary = fixture
        .repository
        .usage_summary(&query)
        .expect("bounded summary");
    assert_eq!(summary.counts.calls, 50);
    assert_eq!(summary.daily[0].local_date, "2026-08-10");
    assert!(summary
        .breakdowns
        .iter()
        .all(|breakdown| breakdown.entries.len() <= 5));
    let details = fixture
        .repository
        .invocation_details(&InvocationDetailQuery {
            session_id: Some("accounting-session".to_string()),
            agent_id: None,
            provider_id: None,
            model_id: None,
            purpose: None,
            quality: None,
            status: None,
            after_id: None,
            limit: 7,
        })
        .expect("bounded details");
    assert_eq!(details.invocations.len(), 7);
    assert!(details.next_cursor.is_some());

    let repository = fixture.repository.clone();
    let query_repository = fixture.repository.clone();
    let query = query.clone();
    let barrier = Arc::new(Barrier::new(2));
    let writer_barrier = barrier.clone();
    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        let mut invocation = accounting_invocation("concurrent-invocation");
        invocation.request_sequence = 51;
        repository
            .start_invocation(&invocation)
            .expect("concurrent start");
        let mut observation = accounting_observation(
            "concurrent-observation",
            "concurrent:source:1",
            MeasurementQuality::Reported,
            2,
        );
        observation.invocation_id = invocation.id.clone();
        repository
            .record_observation(&observation)
            .expect("concurrent observation");
        repository
            .finalize_invocation(
                &invocation.id,
                UsageStatus::Succeeded,
                "2026-08-12T00:00:05Z",
            )
            .expect("concurrent finalize");
    });
    barrier.wait();
    let during = query_repository
        .usage_summary(&query)
        .expect("concurrent query");
    writer.join().expect("concurrent writer");
    let after = query_repository
        .usage_summary(&query)
        .expect("post-ingestion query");
    assert!((50..=51).contains(&during.counts.calls));
    assert_eq!(after.counts.calls, 51);
}
