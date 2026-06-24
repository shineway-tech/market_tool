import type { PlatformAuthSettings } from "./types";

export function defaultPlatformSettings(): PlatformAuthSettings[] {
  return [
    creatorPlatformSetting("xiaohongshu"),
    creatorPlatformSetting("wechat-channels"),
    creatorPlatformSetting("douyin"),
    creatorPlatformSetting("bilibili"),
    creatorPlatformSetting("kuaishou"),
  ];
}

export function creatorPlatformSetting(platformId: string): PlatformAuthSettings {
  return {
    platformId,
    mode: "creator",
    authUrl: "",
    tokenUrl: "",
    profileUrl: "",
    clientId: "",
    clientSecret: "",
    scopes: [],
  };
}
