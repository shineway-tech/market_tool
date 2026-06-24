import type { CopyText } from "../i18n/copy";
import { icon } from "../ui/icons";
import { escapeAttribute } from "../utils/html";

export interface PasswordDraftState {
  currentPassword: string;
  newPassword: string;
  confirmPassword: string;
}

export interface PasswordPageState {
  text: CopyText;
  passwordDraft: PasswordDraftState;
  passwordBusy: boolean;
  inputHints: string;
}

export function renderPasswordPage({
  text,
  passwordDraft,
  passwordBusy,
  inputHints,
}: PasswordPageState) {
  return `
    <section class="page-head">
      <div>
        <h1>${text.passwordSettings}</h1>
      </div>
    </section>
    <section class="single-form-page">
      <article class="settings-card settings-card-form">
        <form class="settings-form" data-settings-form="password">
          <div class="form-grid compact">
            <label>
              <span>${text.currentPassword}</span>
              <input name="currentPassword" type="password" ${inputHints} minlength="6" maxlength="64" value="${escapeAttribute(passwordDraft.currentPassword)}" required />
            </label>
            <label>
              <span>${text.newPassword}</span>
              <input name="newPassword" type="password" ${inputHints} minlength="6" maxlength="64" value="${escapeAttribute(passwordDraft.newPassword)}" required />
            </label>
            <label>
              <span>${text.confirmPassword}</span>
              <input name="confirmPassword" type="password" ${inputHints} minlength="6" maxlength="64" value="${escapeAttribute(passwordDraft.confirmPassword)}" required />
            </label>
          </div>
          <div class="settings-form-actions">
            <button class="primary-btn" type="submit" ${passwordBusy ? "disabled" : ""}>${icon("lock")}${text.changePassword}</button>
          </div>
        </form>
      </article>
    </section>
  `;
}
