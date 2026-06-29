const { KoaApp, Notification } = require('@honeykid/ml');
const router = require('./routers');
const config = require('../config');

const app = new KoaApp();

if (config.dingtalk && config.dingtalk.token) {
  Notification.configure(config.dingtalk.token);
}

app.loadMiddleWares({
  serviceName: 'marketing master api',
  env: config.env,
});
app.use(async (ctx, next) => {
  ctx.set('Access-Control-Allow-Origin', ctx.get('Origin') || '*');
  ctx.set('Access-Control-Allow-Headers', 'Content-Type, X-Token, x-token, x-sign, x-timestamp');
  ctx.set('Access-Control-Allow-Methods', 'GET,POST,PUT,DELETE,OPTIONS');

  if (ctx.method === 'OPTIONS') {
    ctx.status = 204;
    return;
  }

  await next();
});
app.use(router.routes());
app.use(router.allowedMethods());

module.exports = app;
