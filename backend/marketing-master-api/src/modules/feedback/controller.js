const service = require('./service');

class FeedbackController {
  async create(ctx, next) {
    const ret = await service.create(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }
}

module.exports = new FeedbackController();
