const crypto = require('crypto');
const repository = require('./repository');

function toClientFeedback(feedback) {
  return {
    id: feedback.id,
    content: feedback.content,
    contact: feedback.contact,
    status: feedback.status,
    createdAt: feedback.created_at,
  };
}

class FeedbackService {
  async create(userId, entries) {
    const feedback = await repository.create({
      id: crypto.randomUUID(),
      user_id: userId,
      content: entries.content,
      contact: entries.contact || '',
      status: 'new',
    });

    return toClientFeedback(feedback);
  }
}

module.exports = new FeedbackService();
