const fs = require('fs');
const service = require('./service');

class DesktopUpdateController {
  async download(ctx, next) {
    const file = service.getDownloadFile(ctx.params.fileName);

    if (!file) {
      ctx.status = 404;
      await next();
      return;
    }

    ctx.attachment(file.fileName);
    ctx.body = fs.createReadStream(file.filePath);
    await next();
  }

  async check(ctx, next) {
    const update = service.getUpdate(
      ctx.params.target,
      ctx.params.arch,
      ctx.params.currentVersion,
    );

    if (!update) {
      ctx.status = 204;
      await next();
      return;
    }

    ctx.body = update;
    await next();
  }

  async publish(ctx, next) {
    const release = service.publishRelease(ctx.state.entries);

    ctx.setData({
      latest_version: release.latest_version,
      pub_date: release.pub_date,
      platforms: release.platforms,
      updated_at: release.updated_at,
    });
    await next();
  }
}

module.exports = new DesktopUpdateController();
