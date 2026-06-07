use super::*;

const LOG_RETENTION_DAYS: i64 = 10;

mod insert;
mod query;


#[derive(sqlx::FromRow)]
struct FeedbackLogRow {
    ts: i64,
    ts_nanos: i64,
    level: String,
    feedback_log_body: String,
}

fn format_feedback_log_line(
    ts: i64,
    ts_nanos: i64,
    level: &str,
    feedback_log_body: &str,
) -> String {
    let nanos = u32::try_from(ts_nanos).unwrap_or(0);
    let timestamp = match DateTime::<Utc>::from_timestamp(ts, nanos) {
        Some(dt) => dt.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        None => format!("{ts}.{ts_nanos:09}Z"),
    };
    let mut line = format!("{timestamp} {level:>5} {feedback_log_body}");
    if !line.ends_with('\n') {
        line.push('\n');
    }
    line
}

fn push_log_filters<'a>(builder: &mut QueryBuilder<'a, Sqlite>, query: &'a LogQuery) {
    if !query.levels_upper.is_empty() {
        builder.push(" AND UPPER(level) IN (");
        {
            let mut separated = builder.separated(", ");
            for level_upper in &query.levels_upper {
                separated.push_bind(level_upper.as_str());
            }
        }
        builder.push(")");
    }
    if let Some(from_ts) = query.from_ts {
        builder.push(" AND ts >= ").push_bind(from_ts);
    }
    if let Some(to_ts) = query.to_ts {
        builder.push(" AND ts <= ").push_bind(to_ts);
    }
    push_like_filters(builder, "module_path", &query.module_like);
    push_like_filters(builder, "file", &query.file_like);
    let has_thread_filter = !query.thread_ids.is_empty() || query.include_threadless;
    if has_thread_filter {
        builder.push(" AND (");
        let mut needs_or = false;
        for thread_id in &query.thread_ids {
            if needs_or {
                builder.push(" OR ");
            }
            builder.push("thread_id = ").push_bind(thread_id.as_str());
            needs_or = true;
        }
        if query.include_threadless {
            if needs_or {
                builder.push(" OR ");
            }
            builder.push("thread_id IS NULL");
        }
        builder.push(")");
    }
    if let Some(after_id) = query.after_id {
        builder.push(" AND id > ").push_bind(after_id);
    }
    if let Some(search) = query.search.as_ref() {
        builder.push(" AND INSTR(COALESCE(feedback_log_body, ''), ");
        builder.push_bind(search.as_str());
        builder.push(") > 0");
    }
}

fn push_like_filters<'a>(
    builder: &mut QueryBuilder<'a, Sqlite>,
    column: &str,
    filters: &'a [String],
) {
    if filters.is_empty() {
        return;
    }
    builder.push(" AND (");
    for (idx, filter) in filters.iter().enumerate() {
        if idx > 0 {
            builder.push(" OR ");
        }
        builder
            .push(column)
            .push(" LIKE '%' || ")
            .push_bind(filter.as_str())
            .push(" || '%'");
    }
    builder.push(")");
}

#[cfg(test)]
mod tests;
