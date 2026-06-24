const { UnauthorizedError } = require('@honeykid/ml/errors');
const lodash = require('lodash');

const checkAuth = () => async (ctx, next) => {
  if (lodash.isNil(ctx.state.auth_user)) {
    throw new UnauthorizedError('unauthorized');
  }

  await next();
};

module.exports = checkAuth;
