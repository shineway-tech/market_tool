const { validateBody } = require('@honeykid/ml');
const Joi = require('joi');

const account = Joi.string().trim().lowercase().min(3)
  .max(32)
  .pattern(/^[a-z0-9_-]+$/)
  .required()
  .label('账号');

const password = Joi.string().min(6).max(64)
  .required()
  .label('密码');

const captcha = {
  captcha_id: Joi.string().guid({ version: 'uuidv4' }).required()
    .label('验证码 ID'),
  captcha_code: Joi.string().trim().min(4).max(8)
    .required()
    .label('验证码'),
};

const checkRegister = validateBody(Joi.object({
  account,
  password,
  nickname: Joi.string().trim().empty('')
    .max(32)
    .label('昵称'),
  ...captcha,
}), { stripUnknown: true });

const checkLogin = validateBody(Joi.object({
  account,
  password,
  ...captcha,
}), { stripUnknown: true });

const checkUpdateProfile = validateBody(Joi.object({
  nickname: Joi.string().trim().min(1).max(32)
    .required()
    .label('昵称'),
}), { stripUnknown: true });

const checkUpdatePassword = validateBody(Joi.object({
  current_password: password.label('当前密码'),
  new_password: password.label('新密码'),
}), { stripUnknown: true });

module.exports = {
  checkLogin,
  checkRegister,
  checkUpdatePassword,
  checkUpdateProfile,
};
