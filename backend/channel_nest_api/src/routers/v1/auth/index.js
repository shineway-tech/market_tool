const Router = require('koa-router');
const controller = require('./auth');
const {
  checkLogin,
  checkRegister,
  checkUpdatePassword,
  checkUpdateProfile,
} = require('./filter');
const checkAuth = require('../../../middlewares/check_auth');

const router = new Router();

router.get('/captcha', controller.captcha);
router.post('/register', checkRegister, controller.register);
router.post('/login', checkLogin, controller.login);
router.get('/me', checkAuth(), controller.me);
router.put('/profile', checkAuth(), checkUpdateProfile, controller.updateProfile);
router.put('/password', checkAuth(), checkUpdatePassword, controller.updatePassword);

module.exports = router;
