const lodash = require('lodash');
const Jwt = require('../jwt');
const config = require('../../../config');

const setAuthUser = () => async (ctx, next) => {
  const token = ctx.query.token || ctx.request.headers['X-Token'] || ctx.request.headers['x-token'];

  ctx.state.auth_user = null;

  if (!lodash.isEmpty(token)) {
    const userId = Jwt.verifyToken(token);

    if (!lodash.isNil(userId)) {
      ctx.state.auth_user = {
        id: userId,
        source: 'token',
      };
    }
  }

  if (lodash.isNil(ctx.state.auth_user) && config.auth.allow_anonymous_desktop) {
    ctx.state.auth_user = {
      id: 'local-desktop',
      source: 'desktop',
    };
  }

  await next();
};

module.exports = setAuthUser;
