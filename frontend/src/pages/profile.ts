import type { AuthUser } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { icon } from "../ui/icons";
import { escapeAttribute } from "../utils/html";

export interface ProfilePageState {
  text: CopyText;
  currentUser: AuthUser | null;
  profileNickname: string;
  profileBusy: boolean;
  inputHints: string;
}

export function renderProfilePage({
  text,
  currentUser,
  profileNickname,
  profileBusy,
  inputHints,
}: ProfilePageState) {
  return `
    <section class="page-head">
      <div>
        <h1>${text.profileSettings}</h1>
      </div>
    </section>
    <section class="single-form-page">
      <article class="settings-card settings-card-form">
        <form class="settings-form" data-settings-form="profile">
          <div class="form-grid">
            <label>
              <span>${text.account}</span>
              <input name="account" ${inputHints} value="${escapeAttribute(currentUser?.account || "")}" readonly aria-label="${text.accountReadonly}" />
            </label>
            <label>
              <span>${text.nickname}</span>
              <input name="nickname" ${inputHints} maxlength="32" value="${escapeAttribute(profileNickname)}" required />
            </label>
          </div>
          <div class="settings-form-actions">
            <button class="primary-btn" type="submit" ${profileBusy ? "disabled" : ""}>${icon("save")}${text.saveProfile}</button>
          </div>
        </form>
      </article>
    </section>
  `;
}
