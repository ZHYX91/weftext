import { errorUiText, type UiText } from "./ui-locale";
export type HostedPanelConnection = {
    label: UiText;
    connect(): void | Promise<void>;
};
export type HostedPanelFailure = {
    label: UiText;
    message: UiText;
};
/** Independent panels share an authorization lifetime, not a failure outcome. */
export async function connectHostedPanels(current: () => boolean, panels: HostedPanelConnection[]): Promise<HostedPanelFailure[] | null> {
    const failures: HostedPanelFailure[] = [];
    for (const panel of panels) {
        if (!current())
            return null;
        try {
            await panel.connect();
        }
        catch (error) {
            if (!current())
                return null;
            failures.push({ label: panel.label, message: errorUiText(error) });
        }
        if (!current())
            return null;
    }
    return failures;
}
