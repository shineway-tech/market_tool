use super::*;

pub(super) fn normalize_settings(_settings: AuthSettings) -> AuthSettings {
    default_auth_settings()
}

pub(super) fn default_auth_settings() -> AuthSettings {
    AuthSettings {
        platforms: platforms::all()
            .into_iter()
            .map(|platform| creator_platform_auth(platform.id))
            .collect(),
    }
}

pub(super) fn creator_platform_auth(platform_id: &str) -> PlatformAuthSettings {
    PlatformAuthSettings {
        platform_id: platform_id.to_string(),
        mode: AuthMode::Creator,
        auth_url: String::new(),
        token_url: String::new(),
        profile_url: String::new(),
        client_id: String::new(),
        client_secret: String::new(),
        scopes: Vec::new(),
    }
}

pub(super) fn default_platforms() -> Vec<PlatformInfo> {
    platforms::all()
        .into_iter()
        .map(|platform| PlatformInfo {
            id: platform.id.to_string(),
            name: platform.name.to_string(),
            slug: platform.slug.to_string(),
            color: platform.color.to_string(),
            description: platform.description.to_string(),
        })
        .collect()
}
