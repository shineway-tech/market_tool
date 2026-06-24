const { DefaultPlatforms } = require('../src/modules/channel/constants');

function timestamps(DataTypes, sequelize) {
  return {
    created_at: {
      type: DataTypes.DATE,
      allowNull: false,
      defaultValue: sequelize.literal('CURRENT_TIMESTAMP'),
    },
    updated_at: {
      type: DataTypes.DATE,
      allowNull: false,
      defaultValue: sequelize.literal('CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP'),
    },
  };
}

module.exports = {
  async up(queryInterface, DataTypes, { transaction, sequelize }) {
    await queryInterface.createTable('mm_channel_platforms', {
      id: {
        type: DataTypes.BIGINT.UNSIGNED,
        primaryKey: true,
        autoIncrement: true,
      },
      platform_id: {
        type: DataTypes.STRING(64),
        allowNull: false,
        unique: true,
      },
      name: {
        type: DataTypes.STRING(64),
        allowNull: false,
      },
      slug: {
        type: DataTypes.STRING(32),
        allowNull: false,
      },
      color: {
        type: DataTypes.STRING(32),
        allowNull: false,
      },
      description: DataTypes.STRING(255),
      supports_builtin_oauth: {
        type: DataTypes.BOOLEAN,
        allowNull: false,
        defaultValue: true,
      },
      auth_mode: {
        type: DataTypes.STRING(32),
        allowNull: false,
        defaultValue: 'creator',
      },
      sort_no: {
        type: DataTypes.INTEGER,
        allowNull: false,
        defaultValue: 0,
      },
      is_enabled: {
        type: DataTypes.BOOLEAN,
        allowNull: false,
        defaultValue: true,
      },
      config_json: DataTypes.JSON,
      ...timestamps(DataTypes, sequelize),
    }, { transaction });

    await queryInterface.createTable('mm_channel_accounts', {
      id: {
        type: DataTypes.CHAR(36),
        primaryKey: true,
      },
      user_id: {
        type: DataTypes.STRING(64),
        allowNull: false,
        defaultValue: 'local-desktop',
      },
      platform_id: {
        type: DataTypes.STRING(64),
        allowNull: false,
      },
      platform_uid: {
        type: DataTypes.STRING(191),
        allowNull: false,
      },
      nickname: {
        type: DataTypes.STRING(191),
        allowNull: false,
      },
      avatar: DataTypes.TEXT,
      followers: DataTypes.BIGINT.UNSIGNED,
      likes: DataTypes.BIGINT.UNSIGNED,
      status: {
        type: DataTypes.STRING(32),
        allowNull: false,
        defaultValue: 'active',
      },
      access_token: DataTypes.TEXT,
      refresh_token: DataTypes.TEXT,
      token_expires_at: DataTypes.DATE,
      token_payload: DataTypes.JSON,
      login_cookie: DataTypes.TEXT('medium'),
      webview_session_id: DataTypes.STRING(191),
      homepage_url: DataTypes.TEXT,
      last_sync_at: DataTypes.DATE,
      is_deleted: {
        type: DataTypes.BOOLEAN,
        allowNull: false,
        defaultValue: false,
      },
      deleted_at: DataTypes.DATE,
      ...timestamps(DataTypes, sequelize),
    }, { transaction });

    await queryInterface.createTable('mm_channel_auth_tasks', {
      id: {
        type: DataTypes.CHAR(36),
        primaryKey: true,
      },
      user_id: {
        type: DataTypes.STRING(64),
        allowNull: false,
        defaultValue: 'local-desktop',
      },
      platform_id: {
        type: DataTypes.STRING(64),
        allowNull: false,
      },
      mode: {
        type: DataTypes.STRING(32),
        allowNull: false,
      },
      auth_type: {
        type: DataTypes.STRING(32),
        allowNull: false,
        defaultValue: 'oauth',
      },
      url: DataTypes.TEXT,
      callback_url: DataTypes.TEXT,
      status: {
        type: DataTypes.STRING(32),
        allowNull: false,
        defaultValue: 'pending',
      },
      message: DataTypes.STRING(255),
      account_id: DataTypes.CHAR(36),
      expires_at: DataTypes.DATE,
      payload_json: DataTypes.JSON,
      ...timestamps(DataTypes, sequelize),
    }, { transaction });

    await queryInterface.createTable('mm_channel_sync_logs', {
      id: {
        type: DataTypes.BIGINT.UNSIGNED,
        primaryKey: true,
        autoIncrement: true,
      },
      user_id: {
        type: DataTypes.STRING(64),
        allowNull: false,
        defaultValue: 'local-desktop',
      },
      account_id: {
        type: DataTypes.CHAR(36),
        allowNull: false,
      },
      action: {
        type: DataTypes.STRING(64),
        allowNull: false,
      },
      status: {
        type: DataTypes.STRING(32),
        allowNull: false,
      },
      message: DataTypes.STRING(255),
      payload_json: DataTypes.JSON,
      created_at: {
        type: DataTypes.DATE,
        allowNull: false,
        defaultValue: sequelize.literal('CURRENT_TIMESTAMP'),
      },
    }, { transaction });

    await queryInterface.addIndex('mm_channel_accounts', ['user_id', 'platform_id', 'platform_uid'], {
      name: 'idx_mm_channel_accounts_owner_platform_uid',
      transaction,
    });
    await queryInterface.addIndex('mm_channel_auth_tasks', ['user_id', 'platform_id', 'status'], {
      name: 'idx_mm_channel_auth_tasks_owner_platform_status',
      transaction,
    });
    await queryInterface.addIndex('mm_channel_sync_logs', ['account_id', 'created_at'], {
      name: 'idx_mm_channel_sync_logs_account_created_at',
      transaction,
    });

    await queryInterface.bulkInsert('mm_channel_platforms', DefaultPlatforms.map(item => ({
      platform_id: item.platform_id,
      name: item.name,
      slug: item.slug,
      color: item.color,
      description: item.description,
      supports_builtin_oauth: item.supports_builtin_oauth,
      auth_mode: item.auth_mode,
      sort_no: item.sort_no,
      is_enabled: true,
      config_json: JSON.stringify(item.config_json || {}),
      created_at: new Date(),
      updated_at: new Date(),
    })), { transaction });
  },
};
