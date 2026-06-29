import {
  siBilibili,
  siKuaishou,
  siTiktok,
  siXiaohongshu,
  type SimpleIcon,
} from "simple-icons";
import type { PlatformInfo } from "./types";

export const fallbackPlatforms: PlatformInfo[] = [
  {
    id: "xiaohongshu",
    name: "小红书",
    slug: "XHS",
    color: "#ff2442",
    description: "添加并管理多个小红书账号。",
  },
  {
    id: "wechat-channels",
    name: "视频号",
    slug: "WX",
    color: "#ff9f2e",
    description: "添加并管理多个微信视频号账号。",
  },
  {
    id: "douyin",
    name: "抖音",
    slug: "DY",
    color: "#111111",
    description: "添加并管理多个抖音账号。",
  },
  {
    id: "bilibili",
    name: "哔哩哔哩",
    slug: "BILI",
    color: "#00a1d6",
    description: "添加并管理多个 B 站账号。",
  },
  {
    id: "kuaishou",
    name: "快手",
    slug: "KS",
    color: "#ff4906",
    description: "添加并管理多个快手账号。",
  },
];

export type PlatformIcon = SimpleIcon | { title: string; hex: string; markup: string };

export const wechatChannelsIcon: PlatformIcon = {
  title: "视频号",
  hex: "ff9f2e",
  markup: '<svg viewBox="0 0 24 24" role="img" aria-label="视频号"><path d="M11.2 12.1C9.3 7.6 6.3 4.3 4.1 5.8 1.8 7.4 4.4 16 8.9 15.6c1.4-.1 2.2-1.4 2.3-3.5ZM12.8 12.1c1.9-4.5 4.9-7.8 7.1-6.3 2.3 1.6-.3 10.2-4.8 9.8-1.4-.1-2.2-1.4-2.3-3.5Z" fill="none" stroke="currentColor" stroke-width="2.25" stroke-linecap="round" stroke-linejoin="round"/></svg>',
};

export const platformIcons: Record<string, PlatformIcon> = {
  xiaohongshu: siXiaohongshu,
  "wechat-channels": wechatChannelsIcon,
  douyin: siTiktok,
  bilibili: siBilibili,
  kuaishou: siKuaishou,
};
