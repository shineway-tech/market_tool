import type { LanguageMode, ThemeMode } from "../domain/types";
import type { CopyText } from "../i18n/copy";

export interface SettingsPageState {
  text: CopyText;
  language: LanguageMode;
  theme: ThemeMode;
}

export function renderSettingsPage({ text, language, theme }: SettingsPageState) {
  return `
    <section class="page-head">
      <div>
        <h1>${text.settingsTitle}</h1>
      </div>
    </section>
    <section class="settings-grid-page settings-grid-compact">
      <article class="settings-card">
        <div>
          <h2>${text.language}</h2>
        </div>
        <select data-system-setting="language" aria-label="${text.language}">
          <option value="zh" ${language === "zh" ? "selected" : ""}>${text.chinese}</option>
          <option value="en" ${language === "en" ? "selected" : ""}>${text.english}</option>
        </select>
      </article>
      <article class="settings-card">
        <div>
          <h2>${text.theme}</h2>
        </div>
        <select data-system-setting="theme" aria-label="${text.theme}">
          <option value="dark" ${theme === "dark" ? "selected" : ""}>${text.dark}</option>
          <option value="light" ${theme === "light" ? "selected" : ""}>${text.light}</option>
        </select>
      </article>
    </section>
  `;
}
