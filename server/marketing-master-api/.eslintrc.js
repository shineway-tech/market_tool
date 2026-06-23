module.exports = {
  env: {
    es2021: true,
    node: true,
  },
  extends: [
    'airbnb-base',
  ],
  parserOptions: {
    ecmaVersion: 2021,
  },
  rules: {
    camelcase: 'off',
    'class-methods-use-this': 'off',
    'import/no-dynamic-require': 'off',
    'global-require': 'off',
    'no-console': 'off',
    'no-restricted-syntax': 'off',
  },
};
