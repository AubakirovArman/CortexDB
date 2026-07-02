use std::io::{self, BufRead, Write};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::protocol::{empty_result, initialize_result, JsonRpcRequest, JsonRpcResponse};
use crate::tools::{
    tools_list_result, ToolCallResult, ToolExecutor, TOOL_REMEMBER, TOOL_RETRIEVE_CONTEXT,
    TOOL_SEARCH, TOOL_VERIFY_FACT,
};

const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;

pub fn run_stdio<E, R, W>(reader: R, mut writer: W, executor: E) -> io::Result<()>
where
    E: ToolExecutor,
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_message(&executor, &line) {
            serde_json::to_writer(&mut writer, &response)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }
    Ok(())
}

pub fn handle_message<E: ToolExecutor>(executor: &E, line: &str) -> Option<JsonRpcResponse> {
    let request = match serde_json::from_str::<JsonRpcRequest>(line) {
        Ok(request) => request,
        Err(error) => {
            return Some(JsonRpcResponse::error(
                Some(Value::Null),
                PARSE_ERROR,
                format!("parse error: {error}"),
            ))
        }
    };
    let id = request.id.clone();
    let Some(method) = request.method.as_deref() else {
        return respond_error(id, INVALID_REQUEST, "missing method");
    };
    if request.jsonrpc.as_deref() != Some("2.0") {
        return respond_error(id, INVALID_REQUEST, "jsonrpc must be 2.0");
    }
    if method == "notifications/initialized" {
        return None;
    }
    let result = match method {
        "initialize" => Ok(initialize_result()),
        "ping" => Ok(empty_result()),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => call_tool(executor, request.params),
        _ => Err((METHOD_NOT_FOUND, format!("method not found: {method}"))),
    };
    match result {
        Ok(result) => respond_success(id, result),
        Err((code, message)) => respond_error(id, code, message),
    }
}

fn call_tool<E: ToolExecutor>(executor: &E, params: Option<Value>) -> Result<Value, (i32, String)> {
    let params =
        params.ok_or_else(|| (INVALID_PARAMS, "tools/call params are required".to_owned()))?;
    let call: ToolCallParams = serde_json::from_value(params).map_err(|error| {
        (
            INVALID_PARAMS,
            format!("invalid tools/call params: {error}"),
        )
    })?;
    let arguments = call.arguments.unwrap_or_else(|| json!({}));
    let result = match call.name.as_str() {
        TOOL_RETRIEVE_CONTEXT => {
            parse_args(arguments).map(|args| tool_result(executor.retrieve_context(args)))
        }
        TOOL_VERIFY_FACT => {
            parse_args(arguments).map(|args| tool_result(executor.verify_fact(args)))
        }
        TOOL_REMEMBER => parse_args(arguments).map(|args| tool_result(executor.remember(args))),
        TOOL_SEARCH => parse_args(arguments).map(|args| tool_result(executor.search(args))),
        other => Err((INVALID_PARAMS, format!("unknown tool: {other}"))),
    }?;
    serde_json::to_value(result).map_err(|error| {
        (
            INVALID_REQUEST,
            format!("tool result serialization failed: {error}"),
        )
    })
}

fn parse_args<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, (i32, String)> {
    serde_json::from_value(value)
        .map_err(|error| (INVALID_PARAMS, format!("invalid tool arguments: {error}")))
}

fn tool_result(result: Result<ToolCallResult, String>) -> ToolCallResult {
    match result {
        Ok(result) => result,
        Err(error) => ToolCallResult::error_text(error),
    }
}

fn respond_success(id: Option<Value>, result: Value) -> Option<JsonRpcResponse> {
    id.map(|id| JsonRpcResponse::success(Some(id), result))
}

fn respond_error(
    id: Option<Value>,
    code: i32,
    message: impl Into<String>,
) -> Option<JsonRpcResponse> {
    Some(JsonRpcResponse::error(id, code, message))
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    arguments: Option<Value>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{handle_message, run_stdio};
    use crate::tools::{
        RememberArgs, RetrieveContextArgs, SearchArgs, ToolCallResult, ToolExecutor, VerifyFactArgs,
    };

    struct FakeExecutor;

    impl ToolExecutor for FakeExecutor {
        fn retrieve_context(&self, args: RetrieveContextArgs) -> Result<ToolCallResult, String> {
            Ok(ToolCallResult::text(format!("context:{}", args.task)))
        }

        fn verify_fact(&self, args: VerifyFactArgs) -> Result<ToolCallResult, String> {
            Ok(ToolCallResult::text(format!("verify:{}", args.fact)))
        }

        fn remember(&self, args: RememberArgs) -> Result<ToolCallResult, String> {
            Ok(ToolCallResult::text(format!("remember:{}", args.content)))
        }

        fn search(&self, args: SearchArgs) -> Result<ToolCallResult, String> {
            Ok(ToolCallResult::text(format!("search:{}", args.query)))
        }
    }

    #[test]
    fn lists_four_tools() {
        let response = handle_message(
            &FakeExecutor,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
        )
        .unwrap();
        let result = response.result.unwrap();
        assert_eq!(result["tools"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn calls_search_tool() {
        let response = handle_message(
            &FakeExecutor,
            r#"{"jsonrpc":"2.0","id":"s","method":"tools/call","params":{"name":"search","arguments":{"query":"budget"}}}"#,
        )
        .unwrap();
        assert!(response.error.is_none());
        assert_eq!(
            response.result.unwrap()["content"][0]["text"],
            "search:budget"
        );
    }

    #[test]
    fn calls_retrieve_context_tool() {
        let response = handle_message(
            &FakeExecutor,
            r#"{"jsonrpc":"2.0","id":"a","method":"tools/call","params":{"name":"retrieve_context","arguments":{"task":"hello"}}}"#,
        )
        .unwrap();
        assert!(response.error.is_none());
        assert_eq!(
            response.result.unwrap()["content"][0]["text"],
            "context:hello"
        );
    }

    #[test]
    fn returns_method_not_found() {
        let response = handle_message(
            &FakeExecutor,
            r#"{"jsonrpc":"2.0","id":7,"method":"missing"}"#,
        )
        .unwrap();
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[test]
    fn stdio_smoke_writes_one_response_per_line() {
        let input = br#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"verify_fact","arguments":{"fact":"x"}}}
"#;
        let mut output = Vec::new();
        run_stdio(&input[..], &mut output, FakeExecutor).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(value["result"]["content"][0]["text"], json!("verify:x"));
    }
}
