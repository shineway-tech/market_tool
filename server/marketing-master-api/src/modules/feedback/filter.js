const { validateBody } = require('@honeykid/ml');
const Joi = require('joi');

const checkCreate = validateBody(Joi.object({
  content: Joi.string().trim().min(1).max(2000)
    .required()
    .label('反馈内容'),
  contact: Joi.string().trim().empty('').max(191)
    .label('联系方式'),
}), { stripUnknown: true });

module.exports = {
  checkCreate,
};
