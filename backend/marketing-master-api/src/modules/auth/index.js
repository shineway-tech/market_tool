const Router = require('koa-router');
const routes = require('./routes');

const router = new Router();

router.use(routes.routes(), routes.allowedMethods());

module.exports = router;
