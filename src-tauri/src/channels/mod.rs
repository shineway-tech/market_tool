use url::form_urlencoded;

mod bilibili;
mod douyin;
mod kuaishou;
mod wechat_channels;
mod xiaohongshu;

#[derive(Debug, Clone, Copy)]
pub(crate) enum HomepageKind {
    Creator,
    BilibiliSpaceOrSearch,
    KuaishouProfileOrSearch,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DomainRule {
    pub(crate) host: &'static str,
    pub(crate) include_subdomains: bool,
}

#[derive(Debug)]
pub(crate) struct ChannelPlatform {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) slug: &'static str,
    pub(crate) color: &'static str,
    pub(crate) description: &'static str,
    pub(crate) supports_builtin_oauth: bool,
    pub(crate) creator_home_url: &'static str,
    pub(crate) cookie_urls: &'static [&'static str],
    pub(crate) default_cookie_domain: &'static str,
    pub(crate) cookie_domains: &'static [DomainRule],
    pub(crate) login_cookie_names: &'static [&'static str],
    pub(crate) homepage_kind: HomepageKind,
    pub(crate) plugin_auth: bool,
    pub(crate) materialize_avatar: bool,
    pub(crate) avatar_referer: Option<&'static str>,
    pub(crate) avatar_origin: Option<&'static str>,
}

impl ChannelPlatform {
    pub(crate) fn homepage_url(&self, uid: &str, nickname: &str) -> Result<String, String> {
        match self.homepage_kind {
            HomepageKind::Creator => Ok(self.creator_home_url.to_string()),
            HomepageKind::BilibiliSpaceOrSearch => {
                let uid = uid.trim();
                if !uid.is_empty() && uid.chars().all(|ch| ch.is_ascii_digit()) {
                    Ok(format!("https://space.bilibili.com/{}", encode(uid)))
                } else {
                    account_search_url("https://search.bilibili.com/upuser?keyword=", nickname)
                }
            }
            HomepageKind::KuaishouProfileOrSearch => {
                let uid = uid.trim();
                if !uid.is_empty() {
                    Ok(format!("https://www.kuaishou.com/profile/{}", encode(uid)))
                } else {
                    account_search_url("https://www.kuaishou.com/search/author?searchKey=", nickname)
                }
            }
        }
    }

    pub(crate) fn allows_cookie_domain(&self, domain: &str) -> bool {
        let domain = normalize_domain(domain);
        domain.is_empty() || self.cookie_domains.iter().any(|rule| domain_matches(&domain, rule))
    }

    pub(crate) fn is_login_cookie_name(&self, name: &str) -> bool {
        let name = name.trim().to_ascii_lowercase();
        !name.is_empty() && self.login_cookie_names.iter().any(|item| item == &name)
    }
}

pub(crate) fn all() -> [&'static ChannelPlatform; 5] {
    [
        &xiaohongshu::SPEC,
        &wechat_channels::SPEC,
        &douyin::SPEC,
        &bilibili::SPEC,
        &kuaishou::SPEC,
    ]
}

pub(crate) fn platform(platform_id: &str) -> Option<&'static ChannelPlatform> {
    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => Some(&xiaohongshu::SPEC),
        "wechat-channels" => Some(&wechat_channels::SPEC),
        "douyin" => Some(&douyin::SPEC),
        "bilibili" => Some(&bilibili::SPEC),
        "kuaishou" => Some(&kuaishou::SPEC),
        _ => None,
    }
}

pub(crate) fn normalize_platform_id(value: &str) -> String {
    match value {
        "xhs" | "Xhs" | "XHS" => "xiaohongshu".to_string(),
        "wxSph" | "wxsph" | "wechat" => "wechat-channels".to_string(),
        "kwai" | "KWAI" | "Kwai" => "kuaishou".to_string(),
        "BILIBILI" => "bilibili".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn platform_name(platform_id: &str) -> &'static str {
    platform(platform_id)
        .map(|item| item.name)
        .unwrap_or("渠道账号")
}

pub(crate) fn is_plugin_auth_platform(platform_id: &str) -> bool {
    platform(platform_id)
        .map(|item| item.plugin_auth)
        .unwrap_or(false)
}

pub(crate) fn normalize_plugin_login_target(
    platform_id: &str,
    login_target: Option<&str>,
) -> Option<&'static str> {
    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => match login_target {
            Some("home" | "homepage") => Some("home"),
            Some("creator" | "creator-center" | "creation") => Some("creator"),
            _ => Some("creator"),
        },
        _ => None,
    }
}

pub(crate) fn plugin_login_url(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => match login_target {
            Some("home") => Some(xiaohongshu::SPEC.creator_home_url),
            _ => Some(xiaohongshu::SPEC.creator_home_url),
        },
        "wechat-channels" => Some(wechat_channels::SPEC.creator_home_url),
        "douyin" => Some(douyin::SPEC.creator_home_url),
        "bilibili" => Some(bilibili::SPEC.creator_home_url),
        "kuaishou" => Some(kuaishou::LOGIN_URL),
        _ => None,
    }
}

fn account_search_url(prefix: &str, keyword: &str) -> Result<String, String> {
    if keyword.trim().is_empty() {
        return Err("账号缺少主页标识，无法打开主页".to_string());
    }
    Ok(format!("{prefix}{}", encode(keyword.trim())))
}

fn encode(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn normalize_domain(domain: &str) -> String {
    domain.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn domain_matches(domain: &str, rule: &DomainRule) -> bool {
    let host = normalize_domain(rule.host);
    domain == host || (rule.include_subdomains && domain.ends_with(&format!(".{host}")))
}
