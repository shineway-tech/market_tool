const crypto = require('crypto');
const Feedback = require('../models/feedback');

function plain(row) {
  return row && typeof row.toJSON === 'function' ? row.toJSON() : row;
}

function toClientFeedback(feedback) {
  return {
    id: feedback.id,
    content: feedback.content,
    contact: feedback.contact,
    status: feedback.status,
    createdAt: feedback.created_at,
  };
}

class FeedbackLogic {
  static async create(userId, entries) {
    const feedback = plain(await Feedback.create({
      id: crypto.randomUUID(),
      user_id: userId,
      content: entries.content,
      contact: entries.contact || '',
      status: 'new',
    }));

    return toClientFeedback(feedback);
  }
}

module.exports = FeedbackLogic;
