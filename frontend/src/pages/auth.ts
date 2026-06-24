import type { AuthViewMode, CaptchaResponse, ThemeMode } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { icon } from "../ui/icons";
import { escapeAttribute } from "../utils/html";

export interface AuthDraftState {
  account: string;
  password: string;
  nickname: string;
  captchaCode: string;
}

export interface AuthPageState {
  text: CopyText;
  theme: ThemeMode;
  authViewMode: AuthViewMode;
  authDraft: AuthDraftState;
  captcha: CaptchaResponse | null;
  authBusy: boolean;
  inputHints: string;
}

export function renderAuthPage({
  text,
  theme,
  authViewMode,
  authDraft,
  captcha,
  authBusy,
  inputHints,
}: AuthPageState) {
  const isRegister = authViewMode === "register";

  return `
    <div class="auth-shell theme-${theme}">
      <section class="auth-card">
        <div class="auth-brand">
          <div class="brand-mark" aria-hidden="true">M</div>
          <div>
            <strong>${text.appName}</strong>
          </div>
        </div>
        <form class="login-form" data-auth-form="${authViewMode}">
          <div class="auth-form-head">
            <h1>${isRegister ? text.registerTitle : text.loginTitle}</h1>
          </div>
          <label>
            <span>${text.account}</span>
            <input name="account" ${inputHints} placeholder="${text.authAccountPlaceholder}" value="${escapeAttribute(authDraft.account)}" required />
          </label>
          ${
            isRegister
              ? `<label>
                  <span>${text.nickname}</span>
                  <input name="nickname" ${inputHints} placeholder="${text.authNicknamePlaceholder}" value="${escapeAttribute(authDraft.nickname)}" />
                </label>`
              : ""
          }
          <label>
            <span>${text.password}</span>
            <input name="password" type="password" ${inputHints} placeholder="${text.authPasswordPlaceholder}" value="${escapeAttribute(authDraft.password)}" required />
          </label>
          <label>
            <span>${text.captcha}</span>
            <div class="captcha-row">
              <input name="captchaCode" ${inputHints} placeholder="${text.authCaptchaPlaceholder}" value="${escapeAttribute(authDraft.captchaCode)}" required />
              <button class="captcha-img" type="button" data-auth-action="refresh-captcha" title="${text.captchaRefresh}">
                ${captcha ? `<img src="${escapeAttribute(captcha.image)}" alt="${text.captcha}" />` : icon("refresh")}
              </button>
            </div>
          </label>
          <button class="primary-btn auth-submit" type="submit" ${authBusy ? "disabled" : ""}>
            <span class="auth-submit-icon ${authBusy ? "is-visible" : ""}">${icon("refresh")}</span>
            <span>${isRegister ? text.registerSubmit : text.loginSubmit}</span>
          </button>
          <button class="auth-switch" type="button" data-auth-action="${isRegister ? "show-login" : "show-register"}">
            ${isRegister ? text.switchToLogin : text.switchToRegister}
          </button>
        </form>
      </section>
    </div>
  `;
}
