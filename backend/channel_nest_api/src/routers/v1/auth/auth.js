const AuthLogic = require('../../../logics/auth');

class AuthController {
  async captcha(ctx, next) {
    const ret = await AuthLogic.captcha();

    ctx.setData(ret);
    await next();
  }

  async register(ctx, next) {
    const ret = await AuthLogic.register(ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async login(ctx, next) {
    const ret = await AuthLogic.login(ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async me(ctx, next) {
    const ret = await AuthLogic.me(ctx.state.auth_user.id);

    ctx.setData(ret);
    await next();
  }

  async updateProfile(ctx, next) {
    const ret = await AuthLogic.updateProfile(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async updatePassword(ctx, next) {
    const ret = await AuthLogic.updatePassword(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }
}

module.exports = new AuthController();
