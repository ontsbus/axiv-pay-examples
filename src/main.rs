use reqwest::{blocking::Client, header::HeaderValue, redirect::Policy};
use serde_json::{json, Value};
use std::{env, error::Error, fs, io, time::Duration};

const ENDPOINT: &str = "https://pay.axiv.cc/public/mcp";
const DIRECT_ENDPOINT: &str = "https://api.pay.axiv.cc/site/public/mcp";
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn error(message: &str) -> Box<dyn Error> {
    io::Error::other(message).into()
}

fn selected_endpoint(value: Option<String>) -> Result<String> {
    let value = value.unwrap_or_else(|| ENDPOINT.into());
    if value != ENDPOINT && value != DIRECT_ENDPOINT {
        return Err(error(
            "AXIV_PAY_MCP_URL must be one of the two documented Axiv Pay HTTPS endpoints.",
        ));
    }
    Ok(value)
}

struct Mcp {
    http: Client,
    endpoint: String,
    token: Option<HeaderValue>,
    next_id: u64,
}

impl Mcp {
    fn new(endpoint: &str, token: Option<String>) -> Result<Self> {
        let token = token
            .map(|v| {
                if v.is_empty() || v.chars().any(char::is_whitespace) {
                    return Err(error(
                        "AXIV_PAY_API_TOKEN must contain only the raw API token.",
                    ));
                }
                let mut header = HeaderValue::from_str(&format!("Bearer {v}"))
                    .map_err(|_| error("Invalid API token header."))?;
                header.set_sensitive(true);
                Ok(header)
            })
            .transpose()?;
        Ok(Self {
            http: Client::builder()
                .timeout(Duration::from_secs(20))
                .connect_timeout(Duration::from_secs(5))
                .redirect(Policy::none())
                .retry(reqwest::retry::never())
                .build()?,
            endpoint: endpoint.to_owned(),
            token,
            next_id: 0,
        })
    }

    fn rpc(&mut self, method: &str, params: Value, authenticated: bool) -> Result<Value> {
        self.next_id += 1;
        let mut request = self
            .http
            .post(&self.endpoint)
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2026-07-28")
            .json(&json!({"jsonrpc":"2.0","id":self.next_id,"method":method,"params":params}));
        if authenticated {
            request = request.header(
                "Authorization",
                self.token.clone().ok_or_else(|| {
                    error("This payment tool requires your own AXIV_PAY_API_TOKEN.")
                })?,
            );
        }
        let response = request.send().map_err(|_| error(
            "MCP transport failed. Do not blindly repeat a write; preserve the request and idempotency key."))?;
        if !response.status().is_success() {
            return Err(error(&format!(
                "MCP HTTP {}. Redirects are not followed; writes are not retried.",
                response.status()
            )));
        }
        let reply: Value = response.json().map_err(|_| error(
            "Unreadable response. A write may have succeeded; preserve its original request and idempotency key."))?;
        if reply["jsonrpc"] != "2.0" || reply["id"] != self.next_id || reply.get("error").is_some()
        {
            return Err(error(
                "Unexpected JSON-RPC response. Preserve the original request before retrying.",
            ));
        }
        reply
            .get("result")
            .cloned()
            .ok_or_else(|| error("Missing JSON-RPC result."))
    }
}

fn authorize_call(tool: &Value, allow_write: bool) -> Result<bool> {
    let read_only = tool["annotations"]["readOnlyHint"] == true;
    if !read_only && !allow_write {
        return Err(error("This tool is not marked read-only. Review its terms and pass --allow-write to call it."));
    }
    let public = matches!(
        tool["name"].as_str(),
        Some(
            "search_capabilities"
                | "get_capability"
                | "compare_capabilities"
                | "check_payment_route"
        )
    );
    Ok(!public)
}

fn run() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("list");
    if command != "list" && command != "call" {
        return Err(error(
            "Usage: cargo run -- list | call TOOL arguments.json [--allow-write]",
        ));
    }
    if command == "call" && !(args.len() == 3 || (args.len() == 4 && args[3] == "--allow-write")) {
        return Err(error(
            "Usage: cargo run -- call TOOL arguments.json [--allow-write]",
        ));
    }
    let endpoint = selected_endpoint(env::var("AXIV_PAY_MCP_URL").ok())?;
    let mut mcp = Mcp::new(&endpoint, env::var("AXIV_PAY_API_TOKEN").ok())?;
    mcp.rpc(
        "initialize",
        json!({"protocolVersion":"2026-07-28","capabilities":{},
        "clientInfo":{"name":"axiv-pay-rust-example","version":"0.1.0"}}),
        false,
    )?;
    let listing = mcp.rpc("tools/list", json!({}), false)?;
    if command == "list" {
        println!("{}", serde_json::to_string_pretty(&listing)?);
        return Ok(());
    }
    let tool = listing["tools"]
        .as_array()
        .and_then(|tools| tools.iter().find(|t| t["name"] == args[1]))
        .ok_or_else(|| error("Tool not found in the live tools/list response."))?;
    let authenticated = authorize_call(tool, args.len() == 4)?;
    let arguments: Value = serde_json::from_slice(&fs::read(&args[2])?)?;
    if !arguments.is_object() {
        return Err(error("Tool arguments must be a JSON object."));
    }
    let result = mcp.rpc(
        "tools/call",
        json!({"name":args[1],"arguments":arguments}),
        authenticated,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    if result["isError"] == true {
        return Err(error(
            "The MCP tool rejected this call. See the structured result.",
        ));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    #[test]
    fn endpoint_selection_never_sends_a_token_to_an_arbitrary_override() {
        assert_eq!(selected_endpoint(None).unwrap(), ENDPOINT);
        assert_eq!(
            selected_endpoint(Some(DIRECT_ENDPOINT.into())).unwrap(),
            DIRECT_ENDPOINT
        );
        for invalid in [
            "https://evil.example/public/mcp",
            "https://pay.axiv.cc@evil.example/public/mcp",
            "http://api.pay.axiv.cc/site/public/mcp",
        ] {
            assert!(selected_endpoint(Some(invalid.into())).is_err());
        }
    }

    #[test]
    fn writes_require_explicit_opt_in_and_payment_reads_still_require_a_token() {
        let write = json!({"name":"create_payment","annotations":{"readOnlyHint":false}});
        assert!(authorize_call(&write, false).is_err());
        assert!(authorize_call(&write, true).unwrap());
        let private_read = json!({"name":"get_payment_status","annotations":{"readOnlyHint":true}});
        assert!(authorize_call(&private_read, false).unwrap());
        let public_read = json!({"name":"search_capabilities","annotations":{"readOnlyHint":true}});
        assert!(!authorize_call(&public_read, false).unwrap());
    }

    #[test]
    fn redirects_cannot_forward_the_callers_token_to_another_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut buffer = [0u8; 8192];
            let n = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..n]).to_ascii_lowercase();
            assert!(request.contains("authorization: bearer fixture-token"));
            stream.write_all(b"HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:1/leak\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let mut client = Mcp::new(&url, Some("fixture-token".into())).unwrap();
        let result = client.rpc("tools/call", json!({}), true);
        assert!(result.unwrap_err().to_string().contains("307"));
        worker.join().unwrap();
    }
}
