const FeedbackLogic = require('../../../logics/feedback');

class FeedbackController {
  async create(ctx, next) {
    const ret = await FeedbackLogic.create(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }
}

module.exports = new FeedbackController();
