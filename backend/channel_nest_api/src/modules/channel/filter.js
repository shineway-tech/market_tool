const { validateBody, validateQuery } = require('@honeykid/ml');
const Joi = require('joi');
const { PlatformIds } = require('./constants');

const platformId = Joi.string().valid(...Object.values(PlatformIds)).label('平台');

const checkAccountQuery = validateQuery(Joi.object({
  platform_id: platformId.empty(''),
  search_key: Joi.string().trim().empty('').max(64)
    .label('关键字'),
}), { stripUnknown: true });

const checkStartAuth = validateBody(Joi.object({
  platform_id: platformId.required(),
  login_target: Joi.string().trim().empty('').max(64)
    .label('登录目标'),
  callback_url: Joi.string().trim().empty('').max(512)
    .label('回调地址'),
}), { stripUnknown: true });

const checkCompleteAuth = validateBody(Joi.object({
  platform_uid: Joi.string().trim().max(191).required()
    .label('平台账号 ID'),
  nickname: Joi.string().trim().max(191).required()
    .label('昵称'),
  avatar: Joi.string().trim().empty('').max(4096)
    .label('头像'),
  followers: Joi.number().integer().min(0).allow(null)
    .label('粉丝数'),
  likes: Joi.number().integer().min(0).allow(null)
    .label('获赞数'),
  status: Joi.string().valid('active', 'expired', 'pending').default('active').label('状态'),
  login_cookie: Joi.string().empty('').label('登录 Cookie'),
  webview_session_id: Joi.string().trim().empty('').max(191)
    .label('WebView 会话'),
  homepage_url: Joi.string().trim().empty('').max(1024)
    .label('主页地址'),
  token_payload: Joi.object().unknown(true).label('扩展数据'),
}), { stripUnknown: true });

const checkUpsertAccount = validateBody(Joi.object({
  platform_id: platformId.required(),
  platform_uid: Joi.string().trim().max(191).required()
    .label('平台账号 ID'),
  nickname: Joi.string().trim().max(191).required()
    .label('昵称'),
  avatar: Joi.string().trim().empty('').max(4096)
    .label('头像'),
  followers: Joi.number().integer().min(0).allow(null)
    .label('粉丝数'),
  likes: Joi.number().integer().min(0).allow(null)
    .label('获赞数'),
  status: Joi.string().valid('active', 'expired', 'pending').default('active').label('状态'),
  access_token: Joi.string().empty('').label('Access Token'),
  refresh_token: Joi.string().empty('').label('Refresh Token'),
  token_expires_at: Joi.date().allow(null).label('Token 过期时间'),
  token_payload: Joi.object().unknown(true).label('扩展数据'),
  login_cookie: Joi.string().empty('').label('登录 Cookie'),
  webview_session_id: Joi.string().trim().empty('').max(191)
    .label('WebView 会话'),
  homepage_url: Joi.string().trim().empty('').max(1024)
    .label('主页地址'),
}), { stripUnknown: true });

module.exports = {
  checkAccountQuery,
  checkCompleteAuth,
  checkStartAuth,
  checkUpsertAccount,
};
