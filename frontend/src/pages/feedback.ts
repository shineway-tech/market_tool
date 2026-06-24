import type { CopyText } from "../i18n/copy";
import { icon } from "../ui/icons";
import { escapeAttribute, escapeHtml } from "../utils/html";

export interface FeedbackDraftState {
  content: string;
  contact: string;
}

export interface FeedbackPageState {
  text: CopyText;
  feedbackDraft: FeedbackDraftState;
  feedbackBusy: boolean;
  inputHints: string;
}

export function renderFeedbackPage({
  text,
  feedbackDraft,
  feedbackBusy,
  inputHints,
}: FeedbackPageState) {
  return `
    <section class="page-head">
      <div>
        <h1>${text.feedbackTitle}</h1>
      </div>
    </section>
    <section class="single-form-page">
      <article class="settings-card settings-card-form">
        <form class="settings-form" data-feedback-form>
          <label>
            <span>${text.feedbackContent}</span>
            <textarea name="content" ${inputHints} maxlength="2000" placeholder="${text.feedbackContentPlaceholder}" required>${escapeHtml(feedbackDraft.content)}</textarea>
          </label>
          <label>
            <span>${text.feedbackContact}</span>
            <input name="contact" ${inputHints} maxlength="191" placeholder="${text.feedbackContactPlaceholder}" value="${escapeAttribute(feedbackDraft.contact)}" />
          </label>
          <div class="settings-form-actions">
            <button class="primary-btn" type="submit" ${feedbackBusy ? "disabled" : ""}>${icon("send")}${text.feedbackSubmit}</button>
          </div>
        </form>
      </article>
    </section>
  `;
}
