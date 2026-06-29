use super::*;

pub(super) fn browser_page_url(client: &mut DevtoolsClient) -> Result<String, String> {
    let result = client.call(
        "Runtime.evaluate",
        serde_json::json!({
            "expression": "location.href",
            "returnByValue": true,
        }),
    )?;
    Ok(result
        .get("result")
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
}

pub(super) fn wait_for_page_websocket(port: u16, target_url: &str) -> Result<String, String> {
    let started = Instant::now();
    let mut last_error = String::new();
    while started.elapsed() < Duration::from_secs(25) {
        match page_websocket_url(port, target_url) {
            Ok(url) => return Ok(url),
            Err(error) => {
                last_error = error;
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err(format!("浏览器调试端口启动超时: {last_error}"))
}

pub(super) fn create_browser_page(port: u16, url: &str) -> Result<String, String> {
    let encoded = form_urlencoded::byte_serialize(url.as_bytes()).collect::<String>();
    let body = devtools_http(port, "PUT", &format!("/json/new?{encoded}"))?;
    let value = serde_json::from_str::<Value>(&body).map_err(|error| format!("创建浏览器页面失败: {error}"))?;
    value
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| "创建浏览器页面后缺少调试地址".to_string())
}

pub(super) fn page_websocket_url(port: u16, target_url: &str) -> Result<String, String> {
    let body = devtools_http(port, "GET", "/json/list")?;
    let pages = serde_json::from_str::<Value>(&body).map_err(|error| format!("读取浏览器页面失败: {error}"))?;
    let Some(items) = pages.as_array() else {
        return Err("浏览器页面列表格式无效".to_string());
    };
    let target_host = Url::parse(target_url)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_ascii_lowercase()));
    let mut first_page = None;
    let mut blank_page = None;

    for item in items {
        if item.get("type").and_then(Value::as_str) != Some("page") {
            continue;
        }
        let Some(websocket_url) = item.get("webSocketDebuggerUrl").and_then(Value::as_str) else {
            continue;
        };
        let page_url = item.get("url").and_then(Value::as_str).unwrap_or_default();
        if first_page.is_none() {
            first_page = Some(websocket_url.to_string());
        }
        if page_url == "about:blank" && blank_page.is_none() {
            blank_page = Some(websocket_url.to_string());
        }
        if page_url_matches_target(page_url, target_host.as_deref()) {
            return Ok(websocket_url.to_string());
        }
    }

    blank_page
        .or(first_page)
        .ok_or_else(|| "没有找到可控制的浏览器页面".to_string())
}

pub(super) fn browser_websocket_url(port: u16) -> Result<String, String> {
    let body = devtools_http(port, "GET", "/json/version")?;
    let value = serde_json::from_str::<Value>(&body)
        .map_err(|error| format!("读取浏览器版本信息失败: {error}"))?;
    value
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| "浏览器版本信息缺少调试地址".to_string())
}

pub(super) fn browser_debug_port_closed(error: &str) -> bool {
    error.contains("Connection refused")
        || error.contains("connection refused")
        || error.contains("actively refused")
        || error.contains("No connection")
        || error.contains("连接浏览器失败")
}

fn page_url_matches_target(page_url: &str, target_host: Option<&str>) -> bool {
    let Some(target_host) = target_host else {
        return false;
    };
    let Ok(url) = Url::parse(page_url) else {
        return page_url.contains(target_host);
    };
    let Some(host) = url.host_str().map(|value| value.to_ascii_lowercase()) else {
        return false;
    };
    host == target_host || host.ends_with(&format!(".{target_host}"))
}

fn devtools_http(port: u16, method: &str, path: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).map_err(|error| format!("连接浏览器失败: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| format!("设置浏览器读取超时失败: {error}"))?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("请求浏览器失败: {error}"))?;

    let response = read_http_response(&mut stream)?;
    let Some(header_end) = response.windows(4).position(|item| item == b"\r\n\r\n") else {
        return Err("浏览器响应格式无效".to_string());
    };
    let head = String::from_utf8_lossy(&response[..header_end]);
    if !head.contains(" 200 ") {
        return Err(format!("浏览器响应失败: {}", head.lines().next().unwrap_or_default()));
    }
    let raw_body = &response[header_end + 4..];
    let body = if head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        decode_chunked_body(raw_body)?
    } else {
        raw_body.to_vec()
    };
    String::from_utf8(body).map_err(|error| format!("浏览器响应不是 UTF-8: {error}"))
}

fn read_http_response(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut header_end = None;
    let mut expected_total_len = None;
    let started = Instant::now();

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(size) => {
                response.extend_from_slice(&buffer[..size]);
                if header_end.is_none() {
                    header_end = response.windows(4).position(|item| item == b"\r\n\r\n");
                    if let Some(end) = header_end {
                        let head = String::from_utf8_lossy(&response[..end]);
                        if !head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
                            if let Some(content_len) = http_content_length(&head) {
                                expected_total_len = Some(end + 4 + content_len);
                            }
                        }
                    }
                }

                if let Some(total_len) = expected_total_len {
                    if response.len() >= total_len {
                        break;
                    }
                } else if let Some(end) = header_end {
                    let head = String::from_utf8_lossy(&response[..end]);
                    if head.to_ascii_lowercase().contains("transfer-encoding: chunked")
                        && chunked_body_complete(&response[end + 4..])
                    {
                        break;
                    }
                }
            }
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                if header_end.is_some() {
                    break;
                }
                if started.elapsed() > Duration::from_secs(5) {
                    return Err(format!("读取浏览器响应超时: {error}"));
                }
            }
            Err(error) => return Err(format!("读取浏览器响应失败: {error}")),
        }
    }

    Ok(response)
}

fn http_content_length(head: &str) -> Option<usize> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse().ok()
        } else {
            None
        }
    })
}

fn chunked_body_complete(raw: &[u8]) -> bool {
    let mut offset = 0;
    loop {
        let Some(line_end) = raw[offset..].windows(2).position(|item| item == b"\r\n") else {
            return false;
        };
        let line = String::from_utf8_lossy(&raw[offset..offset + line_end]);
        let size_text = line.split(';').next().unwrap_or("").trim();
        let Ok(size) = usize::from_str_radix(size_text, 16) else {
            return false;
        };
        offset += line_end + 2;
        if size == 0 {
            return raw.len() >= offset + 2;
        }
        if raw.len() < offset + size + 2 {
            return false;
        }
        offset += size + 2;
    }
}

fn decode_chunked_body(raw: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut offset = 0;

    loop {
        let Some(line_end) = raw[offset..].windows(2).position(|item| item == b"\r\n") else {
            return Err("浏览器 chunked 响应长度格式无效".to_string());
        };
        let line = String::from_utf8_lossy(&raw[offset..offset + line_end]);
        let size_text = line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|error| format!("浏览器 chunked 响应长度无效: {error}"))?;
        offset += line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        if raw.len() < offset + size + 2 {
            return Err("浏览器 chunked 响应内容不完整".to_string());
        }
        output.extend_from_slice(&raw[offset..offset + size]);
        offset += size + 2;
    }
}

pub(super) struct DevtoolsClient {
    stream: TcpStream,
    next_id: u64,
}

impl DevtoolsClient {
    pub(super) fn connect(websocket_url: &str) -> Result<Self, String> {
        let url = Url::parse(websocket_url).map_err(|error| format!("浏览器调试地址无效: {error}"))?;
        let host = url.host_str().ok_or_else(|| "浏览器调试地址缺少主机".to_string())?;
        let port = url.port().unwrap_or(80);
        let path = if let Some(query) = url.query() {
            format!("{}?{query}", url.path())
        } else {
            url.path().to_string()
        };
        let mut stream = TcpStream::connect((host, port)).map_err(|error| format!("连接浏览器 WebSocket 失败: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| format!("设置浏览器 WebSocket 超时失败: {error}"))?;
        let key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, Uuid::new_v4().as_bytes());
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {key}\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|error| format!("发送浏览器 WebSocket 握手失败: {error}"))?;

        let mut response = Vec::new();
        let mut buffer = [0_u8; 512];
        loop {
            let size = stream
                .read(&mut buffer)
                .map_err(|error| format!("读取浏览器 WebSocket 握手失败: {error}"))?;
            if size == 0 {
                break;
            }
            response.extend_from_slice(&buffer[..size]);
            if response.windows(4).any(|item| item == b"\r\n\r\n") {
                break;
            }
        }
        let response_text = String::from_utf8_lossy(&response);
        if !response_text.contains(" 101 ") {
            return Err(format!(
                "浏览器 WebSocket 握手失败: {}",
                response_text.lines().next().unwrap_or_default()
            ));
        }

        Ok(Self { stream, next_id: 1 })
    }

    pub(super) fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let message = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });
        self.send_text(&message.to_string())?;

        loop {
            let text = self.read_text()?;
            let value = serde_json::from_str::<Value>(&text).map_err(|error| format!("浏览器调试响应无效: {error}"))?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(format!("浏览器调试命令失败 {method}: {error}"));
            }
            return Ok(value.get("result").cloned().unwrap_or_else(|| serde_json::json!({})));
        }
    }

    fn send_text(&mut self, text: &str) -> Result<(), String> {
        let payload = text.as_bytes();
        let mut frame = Vec::new();
        frame.push(0x81);
        if payload.len() <= 125 {
            frame.push(0x80 | payload.len() as u8);
        } else if payload.len() <= u16::MAX as usize {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
        let mask = *Uuid::new_v4().as_bytes().first_chunk::<4>().unwrap_or(&[0, 0, 0, 0]);
        frame.extend_from_slice(&mask);
        for (index, byte) in payload.iter().enumerate() {
            frame.push(byte ^ mask[index % 4]);
        }
        self.stream
            .write_all(&frame)
            .map_err(|error| format!("发送浏览器调试命令失败: {error}"))
    }

    fn read_text(&mut self) -> Result<String, String> {
        loop {
            let mut header = [0_u8; 2];
            self.stream
                .read_exact(&mut header)
                .map_err(|error| format!("读取浏览器调试响应失败: {error}"))?;
            let opcode = header[0] & 0x0f;
            let masked = header[1] & 0x80 != 0;
            let mut len = (header[1] & 0x7f) as u64;
            if len == 126 {
                let mut bytes = [0_u8; 2];
                self.stream
                    .read_exact(&mut bytes)
                    .map_err(|error| format!("读取浏览器调试响应长度失败: {error}"))?;
                len = u16::from_be_bytes(bytes) as u64;
            } else if len == 127 {
                let mut bytes = [0_u8; 8];
                self.stream
                    .read_exact(&mut bytes)
                    .map_err(|error| format!("读取浏览器调试响应长度失败: {error}"))?;
                len = u64::from_be_bytes(bytes);
            }

            let mut mask = [0_u8; 4];
            if masked {
                self.stream
                    .read_exact(&mut mask)
                    .map_err(|error| format!("读取浏览器调试响应掩码失败: {error}"))?;
            }

            let mut payload = vec![0_u8; len as usize];
            self.stream
                .read_exact(&mut payload)
                .map_err(|error| format!("读取浏览器调试响应内容失败: {error}"))?;
            if masked {
                for (index, byte) in payload.iter_mut().enumerate() {
                    *byte ^= mask[index % 4];
                }
            }

            match opcode {
                0x1 => return String::from_utf8(payload).map_err(|error| format!("浏览器调试响应不是 UTF-8: {error}")),
                0x8 => return Err("浏览器调试连接已关闭".to_string()),
                0x9 => self.send_pong(&payload)?,
                _ => {}
            }
        }
    }

    fn send_pong(&mut self, payload: &[u8]) -> Result<(), String> {
        let mut frame = Vec::new();
        frame.push(0x8a);
        frame.push(0x80 | payload.len() as u8);
        let mask = *Uuid::new_v4().as_bytes().first_chunk::<4>().unwrap_or(&[0, 0, 0, 0]);
        frame.extend_from_slice(&mask);
        for (index, byte) in payload.iter().enumerate() {
            frame.push(byte ^ mask[index % 4]);
        }
        self.stream
            .write_all(&frame)
            .map_err(|error| format!("发送浏览器心跳响应失败: {error}"))
    }
}
