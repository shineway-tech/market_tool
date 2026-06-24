import type { PlatformAuthSettings } from "./types";

export function defaultPlatformSettings(): PlatformAuthSettings[] {
  return [
    defaultPlatformSetting("xiaohongshu", "plat/xhs/auth/url/pc"),
    defaultPlatformSetting("wechat-channels", "plat/wxSph/auth/url/pc"),
    defaultPlatformSetting("douyin", "plat/douyin/auth/url"),
    defaultPlatformSetting("bilibili", "plat/bilibili/auth/url/pc"),
    defaultPlatformSetting("kuaishou", "plat/kwai/auth/url/pc"),
  ];
}

export function defaultPlatformSetting(platformId: string, relayPath: string): PlatformAuthSettings {
  return {
    platformId,
    mode: "relay",
    relayPath,
    relayMethod: "GET",
    authUrl: "",
    tokenUrl: "",
    profileUrl: "",
    clientId: "",
    clientSecret: "",
    scopes: [],
  };
}
