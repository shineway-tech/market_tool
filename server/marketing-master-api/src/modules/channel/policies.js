const lodash = require('lodash');
const { ForbiddenError } = require('@honeykid/ml/errors');

const requireOwner = (getOwnerId) => async (ctx, next) => {
  const ownerId = await getOwnerId(ctx);

  if (lodash.isNil(ctx.state.auth_user) || ownerId !== ctx.state.auth_user.id) {
    throw new ForbiddenError('没有权限');
  }

  await next();
};

module.exports = {
  requireOwner,
};
