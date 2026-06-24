const service = require('./service');

class ChannelController {
  async bootstrap(ctx, next) {
    const ret = await service.bootstrap(ctx.state.auth_user.id);

    ctx.setData(ret);
    await next();
  }

  async platforms(ctx, next) {
    const ret = await service.listPlatforms();

    ctx.setData(ret);
    await next();
  }

  async accounts(ctx, next) {
    const ret = await service.listAccounts(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async upsertAccount(ctx, next) {
    const ret = await service.upsertAccount(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async startAuth(ctx, next) {
    const ret = await service.startAuth(ctx.state.auth_user.id, ctx.state.entries);

    ctx.setData(ret);
    await next();
  }

  async authStatus(ctx, next) {
    const ret = await service.getAuthStatus(ctx.state.auth_user.id, ctx.params.task_id);

    ctx.setData(ret);
    await next();
  }

  async completeAuth(ctx, next) {
    const ret = await service.completeAuth(
      ctx.state.auth_user.id,
      ctx.params.task_id,
      ctx.state.entries,
    );

    ctx.setData(ret);
    await next();
  }

  async refreshAccount(ctx, next) {
    const ret = await service.refreshAccount(ctx.state.auth_user.id, ctx.params.account_id);

    ctx.setData(ret);
    await next();
  }

  async deleteAccount(ctx, next) {
    await service.deleteAccount(ctx.state.auth_user.id, ctx.params.account_id);

    ctx.setData({});
    await next();
  }

  async homepage(ctx, next) {
    const ret = await service.getAccountHomepage(ctx.state.auth_user.id, ctx.params.account_id);

    ctx.setData(ret);
    await next();
  }

  async syncRelay(ctx, next) {
    const ret = await service.syncRelayAccounts(ctx.state.auth_user.id);

    ctx.setData(ret);
    await next();
  }
}

module.exports = new ChannelController();
