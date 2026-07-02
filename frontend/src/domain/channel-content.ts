import type { ChannelAccount } from "./types";

export type ContentTab = "overview" | "works" | "comments" | "data";

export type WorkStatus = "published" | "reviewing" | "draft";
export type CommentStatus = "unread" | "replied" | "risk";
export type CommentSentiment = "positive" | "neutral" | "risk";

export interface ChannelWork {
  id: string;
  platformId: string;
  accountId: string;
  title: string;
  workType?: "video" | "article" | string | null;
  publishedAt?: string | null;
  status: WorkStatus;
  coverUrl?: string | null;
  link?: string | null;
  views?: number | null;
  impressions?: number | null;
  likes?: number | null;
  collects?: number | null;
  comments?: number | null;
  shares?: number | null;
  coverClickRate?: string | null;
  avgViewTime?: string | null;
  gainedFollowers?: number | null;
  dataUpdatedAt?: string | null;
  metrics?: ChannelWorkMetric[];
  badges?: string[];
}

export interface ChannelWorkMetric {
  key: string;
  label: string;
  value?: string | null;
}

export interface ChannelOverviewMetric {
  key: string;
  label: string;
  value?: string | null;
  compareLabel?: string | null;
  trend?: string | null;
  tone?: "up" | "down" | string | null;
}

export interface ChannelAccountOverview {
  accountId: string;
  platformId: string;
  periodDays: number;
  metrics: ChannelOverviewMetric[];
  summary?: string | null;
  updatedAt?: string | null;
  syncStatus?: string;
  error?: string | null;
}

export interface ChannelAccountProfileSnapshot {
  accountId: string;
  platformId: string;
  followers?: number | null;
  following?: number | null;
  likes?: number | null;
  lastSyncAt?: string | null;
  updatedAt?: string | null;
  syncStatus?: string;
  error?: string | null;
}

export interface ChannelAccountContent {
  accountId: string;
  platformId: string;
  profile?: ChannelAccountProfileSnapshot | null;
  overviewYesterday?: ChannelAccountOverview | null;
  overviewSeven?: ChannelAccountOverview | null;
  overviewThirty?: ChannelAccountOverview | null;
  overviewNinety?: ChannelAccountOverview | null;
  overviewHistory?: ChannelAccountOverview | null;
  overviewTotal?: ChannelAccountOverview | null;
  latestWork?: ChannelWork | null;
  latestWorkSeven?: ChannelWork | null;
  latestWorkThirty?: ChannelWork | null;
  syncStatus?: string;
  error?: string | null;
}

export interface ChannelWorksPage {
  accountId: string;
  platformId: string;
  pageKey: string;
  workType?: "video" | "article" | string | null;
  nextPageKey?: string | null;
  hasMore: boolean;
  works: ChannelWork[];
  updatedAt?: string | null;
  syncStatus?: string;
  error?: string | null;
}

export interface ChannelComment {
  id: string;
  platformId: string;
  accountId: string;
  workId: string;
  author: string;
  content: string;
  createdAt: string;
  status: CommentStatus;
  sentiment: CommentSentiment;
}

const workTitles = [
  "新品种草视频",
  "达人合作复盘",
  "直播间切片",
  "热点话题跟进",
  "粉丝问答合集",
  "爆款素材测试",
];

const commentContents = [
  "这个组合怎么买更划算？",
  "想看同系列的真实使用反馈。",
  "价格和上次活动一样吗？",
  "已经收藏了，等开播提醒。",
  "这个卖点可以再讲细一点。",
  "评论区有人反馈发货慢，需要跟进。",
];

const authors = ["小夏", "Lynn", "阿川", "Mia", "北北", "Kevin"];
const workStatuses: WorkStatus[] = ["published", "published", "reviewing", "draft"];
const commentStatuses: CommentStatus[] = ["unread", "replied", "unread", "risk"];
const commentSentiments: CommentSentiment[] = ["neutral", "positive", "neutral", "risk"];

export function mockWorksForAccounts(accounts: ChannelAccount[]) {
  return accounts.flatMap((account, accountIndex) => {
    const seed = hashSeed(account.id || account.uid || account.nickname);
    return [0, 1, 2].map((offset): ChannelWork => {
      const value = seed + accountIndex * 17 + offset * 11;
      const title = workTitles[value % workTitles.length];
      return {
        id: `${account.id}-work-${offset + 1}`,
        platformId: account.platformId,
        accountId: account.id,
        title: `${title} #${(value % 9) + 1}`,
        publishedAt: dateBeforeJune30(value + offset * 2),
        status: workStatuses[value % workStatuses.length],
        views: 2400 + (value % 21) * 860 + offset * 420,
        likes: 180 + (value % 17) * 64 + offset * 35,
        comments: 12 + (value % 9) * 7 + offset * 3,
      };
    });
  });
}

export function mockCommentsForAccounts(accounts: ChannelAccount[]) {
  const works = mockWorksForAccounts(accounts);
  return accounts.flatMap((account, accountIndex) => {
    const seed = hashSeed(account.uid || account.id || account.nickname);
    const accountWorks = works.filter((work) => work.accountId === account.id);
    return [0, 1, 2, 3].map((offset): ChannelComment => {
      const value = seed + accountIndex * 13 + offset * 7;
      const work = accountWorks[offset % Math.max(accountWorks.length, 1)];
      return {
        id: `${account.id}-comment-${offset + 1}`,
        platformId: account.platformId,
        accountId: account.id,
        workId: work?.id || `${account.id}-work-1`,
        author: authors[value % authors.length],
        content: commentContents[value % commentContents.length],
        createdAt: dateBeforeJune30(value + offset),
        status: commentStatuses[value % commentStatuses.length],
        sentiment: commentSentiments[value % commentSentiments.length],
      };
    });
  });
}

function hashSeed(value: string) {
  return Array.from(value || "channel").reduce((sum, char) => sum + char.charCodeAt(0), 0);
}

function dateBeforeJune30(seed: number) {
  const day = 29 - (seed % 18);
  const hour = 9 + (seed % 9);
  return new Date(Date.UTC(2026, 5, day, hour, 30)).toISOString();
}
