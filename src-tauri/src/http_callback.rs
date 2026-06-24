use super::*;

#[derive(Debug)]
pub(super) struct HttpRequest {
    pub(super) path: String,
    pub(super) query: HashMap<String, String>,
}

pub(super) fn parse_http_request(raw: &str) -> Result<HttpRequest, String> {
    let (head, _body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| "HTTP 请求格式无效".to_string())?;
    let mut lines = head.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "HTTP 请求缺少请求行".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let _method = request_parts.next().unwrap_or_default();
    let target = request_parts.next().unwrap_or("/");
    let mut target_parts = target.splitn(2, '?');
    let path = target_parts.next().unwrap_or("/").to_string();
    let query = target_parts
        .next()
        .map(parse_query)
        .unwrap_or_else(HashMap::new);
    Ok(HttpRequest {
        path,
        query,
    })
}

pub(super) fn parse_query(value: &str) -> HashMap<String, String> {
    form_urlencoded::parse(value.as_bytes())
        .into_owned()
        .collect::<HashMap<String, String>>()
}
