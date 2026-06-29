const ChannelLogic = require('../../../logics/channel');

class ChannelController {
  async bootstrap(ctx, next) {
    const ret = await ChannelLogic.bootstrap(ctx.state.auth_user.id);

    ctx.setData(ret);
    await next();
  }

  async platforms(ctx, next) {
    const ret = await ChannelLogic.listPlatforms();

    ctx.setData(ret);
    await next();
  }

  async accounts(ctx, next) {
    const ret = await ChannelLogic.listAccounts(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async upsertAccount(ctx, next) {
    const ret = await ChannelLogic.upsertAccount(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async startAuth(ctx, next) {
    const ret = await ChannelLogic.startAuth(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async authStatus(ctx, next) {
    const ret = await ChannelLogic.getAuthStatus(ctx.state.auth_user.id, ctx.params.task_id);

    ctx.setData(ret);
    await next();
  }

  async completeAuth(ctx, next) {
    const ret = await ChannelLogic.completeAuth(
      ctx.state.auth_user.id,
      ctx.params.task_id,
      ctx.state.entries,
    );

    ctx.setData(ret);
    await next();
  }

  async refreshAccount(ctx, next) {
    const ret = await ChannelLogic.refreshAccount(ctx.state.auth_user.id, ctx.params.account_id);

    ctx.setData(ret);
    await next();
  }

  async deleteAccount(ctx, next) {
    await ChannelLogic.deleteAccount(ctx.state.auth_user.id, ctx.params.account_id);

    ctx.setData({});
    await next();
  }

  async homepage(ctx, next) {
    const ret = await ChannelLogic.getAccountHomepage(
      ctx.state.auth_user.id,
      ctx.params.account_id,
    );

    ctx.setData(ret);
    await next();
  }
}

module.exports = new ChannelController();
