import type { LanguageMode } from "./types";
import type { ReleaseHistoryEntry } from "../pages/releases";

export function releaseHistoryForLanguage(language: LanguageMode): ReleaseHistoryEntry[] {
  if (language === "en") {
    return [
      {
        version: "1.0.3",
        date: "2026.06.26",
        icon: "spark",
        sections: [
          {
            icon: "home",
            title: "Navigation",
            items: ["Added a compact left icon rail", "Added a home shortcut back to channel management"],
          },
          {
            icon: "refresh",
            title: "Fixes",
            items: ["Fixed Windows creator homepage WebView windows not closing from the title bar", "Kept Windows release output ready for portable use and updater packaging"],
          },
        ],
      },
      {
        version: "1.0.1",
        date: "2026.06.24",
        icon: "spark",
        sections: [
          {
            icon: "layers",
            title: "Project Structure",
            items: ["Moved frontend and backend into separate folders", "Split frontend pages, UI components, services, domain data, and styles"],
          },
          {
            icon: "refresh",
            title: "Fixes",
            items: ["Fixed profile and password update requests failing with Load failed", "Kept Tauri build paths aligned with the new frontend folder"],
          },
        ],
      },
      {
        version: "1.0.0",
        date: "2026.06.23",
        icon: "spark",
        sections: [
          {
            icon: "lock",
            title: "Account Access",
            items: ["Password sign-in and registration", "Captcha verification", "Profile editing and password change"],
          },
          {
            icon: "layers",
            title: "Channel Management",
            items: ["Xiaohongshu, WeChat Channels, Douyin, Bilibili, and Kuaishou", "Multiple accounts per platform", "Avatar, nickname, followers, and status display"],
          },
          {
            icon: "refresh",
            title: "Account Operations",
            items: ["Refresh account data", "Delete connected accounts", "Open the platform creator homepage"],
          },
          {
            icon: "settings",
            title: "Client Settings",
            items: ["Chinese and English language switch", "Dark and light themes", "Local JSON configuration"],
          },
          {
            icon: "message",
            title: "Feedback",
            items: ["Submit feedback from the client", "Store feedback in the local service"],
          },
        ],
      },
    ];
  }

  return [
    {
      version: "1.0.3",
      date: "2026.06.26",
      icon: "spark",
      sections: [
        {
          icon: "home",
          title: "导航体验",
          items: ["增加左侧窄图标栏", "增加主页入口，可从设置、公告、反馈等页面回到渠道管理"],
        },
        {
          icon: "refresh",
          title: "问题修复",
          items: ["修复 Windows 平台创作者主页 WebView 无法通过标题栏关闭的问题", "Windows 发布流程同时保留免安装版本和自动更新打包资产"],
        },
      ],
    },
    {
      version: "1.0.1",
      date: "2026.06.24",
      icon: "spark",
      sections: [
        {
          icon: "layers",
          title: "项目结构",
          items: ["前端和后端拆分到独立目录", "前端页面、组件、服务、领域数据和样式完成模块化拆分"],
        },
        {
          icon: "refresh",
          title: "问题修复",
          items: ["修复修改个人信息、修改密码时出现 Load failed 的问题", "同步调整 Tauri 打包路径以适配新的前端目录"],
        },
      ],
    },
    {
      version: "1.0.0",
      date: "2026.06.23",
      icon: "spark",
      sections: [
        {
          icon: "lock",
          title: "账号体系",
          items: ["账号密码登录与注册", "验证码校验", "个人信息和密码修改"],
        },
        {
          icon: "layers",
          title: "渠道管理",
          items: ["小红书、视频号、抖音、哔哩哔哩、快手授权", "同一平台支持多个账号", "展示头像、昵称、粉丝数和状态"],
        },
        {
          icon: "refresh",
          title: "账号操作",
          items: ["刷新账号数据", "删除已授权账号", "打开对应平台创作者主页"],
        },
        {
          icon: "settings",
          title: "客户端设置",
          items: ["中文 / 英文切换", "深色 / 浅色主题", "本地 JSON 配置"],
        },
        {
          icon: "message",
          title: "意见反馈",
          items: ["客户端内提交反馈", "反馈内容保存到本地服务"],
        },
      ],
    },
  ];
}
