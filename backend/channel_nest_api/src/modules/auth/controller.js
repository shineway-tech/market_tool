const service = require('./service');

class AuthController {
  async captcha(ctx, next) {
    const ret = await service.captcha();

    ctx.setData(ret);
    await next();
  }

  async register(ctx, next) {
    const ret = await service.register(ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async login(ctx, next) {
    const ret = await service.login(ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async me(ctx, next) {
    const ret = await service.me(ctx.state.auth_user.id);

    ctx.setData(ret);
    await next();
  }

  async updateProfile(ctx, next) {
    const ret = await service.updateProfile(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async updatePassword(ctx, next) {
    const ret = await service.updatePassword(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }
}

module.exports = new AuthController();
