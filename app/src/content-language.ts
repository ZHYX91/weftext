import { message, setUiText, setUiAttr, type UiLocale } from "./ui-locale";
/** An explicit content choice, independent of the reader's interface preference. */
export const DEFAULT_CONTENT_LOCALE: UiLocale = "zh-CN";
export function isContentLocale(value: unknown): value is UiLocale { return value === "zh-CN" || value === "en-US"; }
export function contentLanguageControl(kind: "output" | "annotation", selected: UiLocale, changed: (value: UiLocale) => void): HTMLLabelElement {
    const label = document.createElement("label"), title = document.createElement("span"), select = document.createElement("select"), help = document.createElement("span");
    label.className = "knowledge-field content-language";
    select.className = "data-input";
    const text = message(kind === "output" ? "contentLanguage.output" : "contentLanguage.annotation");
    setUiText(title, text);
    setUiAttr(select, "aria-label", text);
    select.dataset.controlRole = `${kind}-content-language`;
    for (const [value, key] of [["zh-CN", "locale.zhCN"], ["en-US", "locale.enUS"]] as const) {
        const option = document.createElement("option");
        option.value = value;
        setUiText(option, message(key));
        select.append(option);
    }
    select.value = selected;
    select.onchange = () => { if (isContentLocale(select.value))
        changed(select.value); };
    help.className = "data-help";
    setUiText(help, message("contentLanguage.help"));
    label.append(title, select, help);
    return label;
}
