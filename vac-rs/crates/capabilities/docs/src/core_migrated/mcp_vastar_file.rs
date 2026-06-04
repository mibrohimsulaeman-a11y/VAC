//! Bridges Apps SDK-style `vastar/fileParams` metadata into VAC's MCP flow.
//!
//! Strategy:
//! - Inspect `_meta["vastar/fileParams"]` to discover which tool arguments are
//!   file inputs.
//! - At tool execution time, upload those local files to Vastar file storage
//!   and rewrite only the declared arguments into the provided-file payload
//!   shape expected by the downstream Apps tool.
//!
//! Model-visible schema masking is owned by `vac-mcp` alongside MCP tool
//! inventory, so this module only handles the execution-time argument rewrite.

use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use serde_json::Value as JsonValue;
use vac_login::VACAuth;

pub(crate) async fn rewrite_mcp_tool_arguments_for_vastar_files(
    sess: &Session,
    turn_context: &TurnContext,
    arguments_value: Option<JsonValue>,
    vastar_file_input_params: Option<&[String]>,
) -> Result<Option<JsonValue>, String> {
    let Some(vastar_file_input_params) = vastar_file_input_params else {
        return Ok(arguments_value);
    };

    let Some(arguments_value) = arguments_value else {
        return Ok(None);
    };
    let Some(arguments) = arguments_value.as_object() else {
        return Ok(Some(arguments_value));
    };
    let auth = sess.services.auth_manager.auth().await;
    let mut rewritten_arguments = arguments.clone();

    for field_name in vastar_file_input_params {
        let Some(value) = arguments.get(field_name) else {
            continue;
        };
        let Some(uploaded_value) =
            rewrite_argument_value_for_vastar_files(turn_context, auth.as_ref(), field_name, value)
                .await?
        else {
            continue;
        };
        rewritten_arguments.insert(field_name.clone(), uploaded_value);
    }

    if rewritten_arguments == *arguments {
        return Ok(Some(arguments_value));
    }

    Ok(Some(JsonValue::Object(rewritten_arguments)))
}

async fn rewrite_argument_value_for_vastar_files(
    turn_context: &TurnContext,
    auth: Option<&VACAuth>,
    field_name: &str,
    value: &JsonValue,
) -> Result<Option<JsonValue>, String> {
    match value {
        JsonValue::String(path_or_file_ref) => {
            let rewritten = build_uploaded_local_argument_value(
                turn_context,
                auth,
                field_name,
                /*index*/ None,
                path_or_file_ref,
            )
            .await?;
            Ok(Some(rewritten))
        }
        JsonValue::Array(values) => {
            let mut rewritten_values = Vec::with_capacity(values.len());
            for (index, item) in values.iter().enumerate() {
                let Some(path_or_file_ref) = item.as_str() else {
                    return Ok(None);
                };
                let rewritten = build_uploaded_local_argument_value(
                    turn_context,
                    auth,
                    field_name,
                    Some(index),
                    path_or_file_ref,
                )
                .await?;
                rewritten_values.push(rewritten);
            }
            Ok(Some(JsonValue::Array(rewritten_values)))
        }
        _ => Ok(None),
    }
}

async fn build_uploaded_local_argument_value(
    turn_context: &TurnContext,
    _auth: Option<&VACAuth>,
    field_name: &str,
    index: Option<usize>,
    file_path: &str,
) -> Result<JsonValue, String> {
    let _resolved_path = turn_context.resolve_path(Some(file_path.to_string()));
    let target = match index {
        Some(index) => format!("{field_name}[{index}]"),
        None => field_name.to_string(),
    };
    Err(crate::cloud_account_disabled::disabled_feature_message(&format!(
        "VAC Apps cloud file upload for `{target}` from `{file_path}`"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::tests::make_session_and_context;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn vastar_file_argument_rewrite_requires_declared_file_params() {
        let (session, turn_context) = make_session_and_context().await;
        let arguments = Some(serde_json::json!({
            "file": "/tmp/vac-smoke-file.txt"
        }));

        let rewritten = rewrite_mcp_tool_arguments_for_vastar_files(
            &session,
            &Arc::new(turn_context),
            arguments.clone(),
            /*vastar_file_input_params*/ None,
        )
        .await
        .expect("rewrite should succeed");

        assert_eq!(rewritten, arguments);
    }

    #[tokio::test]
    async fn build_uploaded_local_argument_value_uploads_local_file_path() {
        let (_, turn_context) = make_session_and_context().await;
        let dir = tempdir().expect("temp dir");
        let local_path = dir.path().join("file_report.csv");
        tokio::fs::write(&local_path, b"hello")
            .await
            .expect("write local file");

        // Cloud upload is intentionally disabled in the local coding-agent build.
        // build_uploaded_local_argument_value must fail-closed with the disabled reason.
        let err = build_uploaded_local_argument_value(
            &turn_context,
            /*auth*/ None,
            "file",
            /*index*/ None,
            "file_report.csv",
        )
        .await
        .expect_err("cloud upload is disabled; should return disabled-feature error");

        assert!(
            err.contains("legacy ChatGPT-account backend integration is disabled"),
            "error should mention disabled backend: {err}"
        );
        assert!(
            err.contains("file_report.csv"),
            "error should mention the file: {err}"
        );
    }

    #[tokio::test]
    async fn rewrite_argument_value_for_vastar_files_rewrites_scalar_path() {
        let (_, turn_context) = make_session_and_context().await;
        let dir = tempdir().expect("temp dir");
        let local_path = dir.path().join("file_report.csv");
        tokio::fs::write(&local_path, b"hello")
            .await
            .expect("write local file");

        // Cloud upload is intentionally disabled.
        let err = rewrite_argument_value_for_vastar_files(
            &turn_context,
            /*auth*/ None,
            "file",
            &serde_json::json!("file_report.csv"),
        )
        .await
        .expect_err("cloud upload is disabled; should return disabled-feature error");

        assert!(
            err.contains("legacy ChatGPT-account backend integration is disabled"),
            "error should mention disabled backend: {err}"
        );
    }

    #[tokio::test]
    async fn rewrite_argument_value_for_vastar_files_rewrites_array_paths() {
        let (_, turn_context) = make_session_and_context().await;
        let dir = tempdir().expect("temp dir");
        tokio::fs::write(dir.path().join("one.csv"), b"one")
            .await
            .expect("write first local file");
        tokio::fs::write(dir.path().join("two.csv"), b"two")
            .await
            .expect("write second local file");

        // Cloud upload is intentionally disabled.
        let err = rewrite_argument_value_for_vastar_files(
            &turn_context,
            /*auth*/ None,
            "files",
            &serde_json::json!(["one.csv", "two.csv"]),
        )
        .await
        .expect_err("cloud upload is disabled; should return disabled-feature error");

        assert!(
            err.contains("legacy ChatGPT-account backend integration is disabled"),
            "error should mention disabled backend: {err}"
        );
    }

    #[tokio::test]
    async fn rewrite_mcp_tool_arguments_for_vastar_files_surfaces_upload_failures() {
        let (mut session, turn_context) = make_session_and_context().await;
        session.services.auth_manager = crate::test_support::auth_manager_from_auth(
            VACAuth::create_dummy_chatgpt_auth_for_testing(),
        );
        let error = rewrite_mcp_tool_arguments_for_vastar_files(
            &session,
            &turn_context,
            Some(serde_json::json!({
                "file": "/definitely/missing/file.csv",
            })),
            Some(&["file".to_string()]),
        )
        .await
        .expect_err("cloud upload is disabled; should return disabled-feature error");

        // The upload backend is disabled; the error mentions both the disabled reason
        // and the field name.
        assert!(
            error.contains("legacy ChatGPT-account backend integration is disabled"),
            "error should mention disabled backend: {error}"
        );
        assert!(error.contains("file"), "error should mention the field: {error}");
    }
}
