// SPDX-License-Identifier: AGPL-3.0-or-later
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use serde_json::{json, Value};
pub const MAX_RESPONSE: usize = 256 * 1024;
pub const MAX_REQUEST: usize = 14 * 1024;
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub host: String,
    pub authority: String,
    pub port: u16,
    pub base: String,
}
impl Endpoint {
    pub fn parse(raw: &str) -> Result<Self, &'static str> {
        let raw = raw.trim();
        let rest = raw
            .strip_prefix("https://")
            .ok_or("Please enter an HTTPS API URL.")?;
        if !rest.is_ascii()
            || rest.bytes().any(|b| b <= 32 || b == 127)
            || rest.contains(['@', '#', '?', '\\'])
        {
            return Err("Invalid API URL.");
        }
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let (host, port) = match authority.split_once(':') {
            Some((h, p)) => (h, p.parse::<u16>().map_err(|_| "Invalid port.")?),
            None => (authority, 443),
        };
        if port == 0
            || host.is_empty()
            || host.len() > 253
            || host.split('.').any(|l| {
                l.is_empty()
                    || l.len() > 63
                    || l.starts_with('-')
                    || l.ends_with('-')
                    || !l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        {
            return Err("Invalid hostname (IPv6 is not supported yet).");
        }
        let base = format!("/{}", path.trim_end_matches('/'));
        Ok(Self {
            host: host.to_string(),
            authority: authority.to_string(),
            port,
            base: base.trim_end_matches('/').to_string(),
        })
    }
}
#[derive(Clone)]
pub struct Message {
    pub role: &'static str,
    pub text: String,
}
pub fn request(
    ep: &Endpoint,
    key: &str,
    model: &str,
    messages: &[Message],
    models: bool,
) -> Result<Vec<u8>, &'static str> {
    if key.is_empty() || key.len() > 4096 || !key.bytes().all(|b| b >= 33 && b <= 126) {
        return Err("API key is missing or contains invalid characters.");
    }
    if !models && (model.trim().is_empty() || model.len() > 256) {
        return Err("Please enter a model ID.");
    }
    let body = if models {
        String::new()
    } else {
        let rows: Vec<Value> = messages
            .iter()
            .map(|m| json!({"role":m.role,"content":m.text}))
            .collect();
        json!({"model":model.trim(),"messages":rows,"stream":false}).to_string()
    };
    let suffix = if models {
        "/models"
    } else {
        "/chat/completions"
    };
    let method = if models { "GET" } else { "POST" };
    let req = format!("{method} {}{suffix} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {key}\r\nContent-Type: application/json\r\nAccept: application/json\r\nAccept-Encoding: identity\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", ep.base, ep.authority, body.len());
    if req.len() > MAX_REQUEST {
        return Err("Chat is too long. Please start a new chat.");
    }
    Ok(req.into_bytes())
}
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}
pub fn response(raw: &[u8]) -> Result<Option<Response>, &'static str> {
    if raw.len() > MAX_RESPONSE {
        return Err("Response is too large.");
    }
    let Some(at) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
        return Ok(None);
    };
    if at > 16384 {
        return Err("HTTP headers are too large.");
    }
    let header = core::str::from_utf8(&raw[..at]).map_err(|_| "Invalid HTTP response.")?;
    let mut lines = header.split("\r\n");
    let mut statusline = lines
        .next()
        .ok_or("Missing HTTP status.")?
        .split_whitespace();
    if !matches!(statusline.next(), Some("HTTP/1.1" | "HTTP/1.0")) {
        return Err("Invalid HTTP status.");
    }
    let status = statusline
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or("Invalid HTTP status.")?;
    let mut length = None;
    let mut chunked = false;
    for l in lines {
        let (k, v) = l.split_once(':').ok_or("Invalid HTTP header.")?;
        let v = v.trim();
        if k.eq_ignore_ascii_case("content-length") {
            let n = v.parse::<usize>().map_err(|_| "Invalid response length.")?;
            if n > MAX_RESPONSE || length.is_some() {
                return Err("Invalid response length.");
            }
            length = Some(n);
        }
        if k.eq_ignore_ascii_case("transfer-encoding") {
            if !v.eq_ignore_ascii_case("chunked") {
                return Err("Unsupported transfer encoding.");
            }
            chunked = true;
        }
        if k.eq_ignore_ascii_case("content-encoding") && !v.eq_ignore_ascii_case("identity") {
            return Err("Compressed API responses are not supported.");
        }
    }
    if chunked && length.is_some() {
        return Err("Ambiguous HTTP response.");
    }
    let payload = &raw[at + 4..];
    let body = if chunked {
        let Some(b) = chunks(payload)? else {
            return Ok(None);
        };
        b
    } else if let Some(n) = length {
        if payload.len() < n {
            return Ok(None);
        }
        payload[..n].to_vec()
    } else {
        // APIs without explicit HTTP framing can finish only when a complete JSON value arrived.
        if serde_json::from_slice::<Value>(payload).is_err() {
            return Ok(None);
        }
        payload.to_vec()
    };
    Ok(Some(Response { status, body }))
}
fn chunks(mut data: &[u8]) -> Result<Option<Vec<u8>>, &'static str> {
    let mut out = Vec::new();
    loop {
        let Some(p) = data.windows(2).position(|w| w == b"\r\n") else {
            return Ok(None);
        };
        let line = core::str::from_utf8(&data[..p]).map_err(|_| "Invalid chunk length.")?;
        let size = usize::from_str_radix(line.split(';').next().unwrap_or(""), 16)
            .map_err(|_| "Invalid chunk length.")?;
        data = &data[p + 2..];
        if size == 0 {
            if data.starts_with(b"\r\n") || data.windows(4).any(|w| w == b"\r\n\r\n") {
                return Ok(Some(out));
            }
            return Ok(None);
        }
        if size > MAX_RESPONSE.saturating_sub(out.len()) {
            return Err("Response is too large.");
        }
        if data.len() < size + 2 {
            return Ok(None);
        }
        if &data[size..size + 2] != b"\r\n" {
            return Err("Invalid chunk boundary.");
        }
        out.extend_from_slice(&data[..size]);
        data = &data[size + 2..];
    }
}
pub fn answer(r: Response, models: bool) -> Result<String, String> {
    if r.status < 200 || r.status >= 300 {
        // Provider error bodies may echo credentials or the prompt; never display them.
        return Err(match r.status {
            401 | 403 => "Access denied: check your API key and permissions.".into(),
            404 => "API path or model not found.".into(),
            429 => "Provider limit reached. Please try again later.".into(),
            _ => format!("API request failed (HTTP {}).", r.status),
        });
    }
    let v: Value = serde_json::from_slice(&r.body)
        .map_err(|_| "The API returned invalid JSON.".to_string())?;
    if models {
        let list = v["data"]
            .as_array()
            .ok_or("No model list returned. Please enter a model ID manually.")?;
        let names: Vec<&str> = list
            .iter()
            .filter_map(|x| x["id"].as_str())
            .take(80)
            .collect();
        if names.is_empty() {
            return Err("No models available. Please enter a model ID manually.".into());
        }
        return Ok(format!(
            "Connected. Available models:\n{}",
            names.join("\n")
        ));
    }
    v["choices"][0]["message"]["content"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .ok_or("The model did not return a text response.".into())
}
