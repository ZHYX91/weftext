import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, errorUiText, UiError } from "./ui-locale";
import { uiTextValue, type UiText } from "./ui-locale";
/** Personal successful navigation only. A failed open never advances history. */
export class EverydayNavigation {
    private visits: string[] = [];
    private position = -1;
    private recentKeys: string[] = [];
    get current(): string | undefined { return this.visits[this.position]; }
    get recent(): readonly string[] { return this.recentKeys; }
    target(direction: -1 | 1): string | undefined { return this.visits[this.position + direction]; }
    visit(key: string): void {
        if (this.current !== key) {
            this.visits = this.visits.slice(0, this.position + 1);
            this.visits.push(key);
            if (this.visits.length > 128)
                this.visits.shift();
            this.position = this.visits.length - 1;
        }
        this.recentKeys = [key, ...this.recentKeys.filter(item => item !== key)].slice(0, 32);
    }
    arrive(direction: -1 | 1, key: string): void {
        if (this.target(direction) !== key)
            throw new UiError(uiMessage("everydayState.navigationRecordChanged"));
        this.position += direction;
        this.recentKeys = [key, ...this.recentKeys.filter(item => item !== key)].slice(0, 32);
    }
    clear(): void { this.visits = []; this.position = -1; this.recentKeys = []; }
}
export interface EverydayEntry {
    key: string;
    title: string;
    detail: UiText;
    unavailable?: UiText;
    open(): Promise<void>;
}
export interface EverydayCommand {
    id: string;
    label: UiText;
    detail?: UiText;
    unavailable?: UiText;
    run(): void | Promise<void>;
}
export function matchEveryday<T extends {
    title?: string;
    label?: UiText;
    detail?: UiText;
}>(items: readonly T[], text: string): T[] {
    const terms = text.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
    return items.filter(item => terms.every(term => `${uiTextValue(item.title ?? item.label ?? "")} ${uiTextValue(item.detail ?? "")}`.toLocaleLowerCase().includes(term)));
}
