const { validateBody } = require('@honeykid/ml');
const Joi = require('joi');

const checkPublishRelease = validateBody(Joi.object({
  latest_version: Joi.string().trim().pattern(/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/)
    .required()
    .label('版本号'),
  pub_date: Joi.string().trim().isoDate()
    .required()
    .label('发布时间'),
  notes: Joi.string().trim().empty('').max(5000)
    .default('')
    .label('更新说明'),
  platforms: Joi.object()
    .pattern(
      /^[a-z0-9_-]+-[a-z0-9_]+$/,
      Joi.object({
        url: Joi.string().trim().uri({ scheme: ['https'] })
          .required()
          .label('下载地址'),
        signature: Joi.string().trim().min(1)
          .required()
          .label('更新签名'),
      }),
    )
    .min(1)
    .required()
    .label('平台更新信息'),
}), { stripUnknown: true });

module.exports = {
  checkPublishRelease,
};
