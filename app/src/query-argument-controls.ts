import { action, emptyNativeValue, formatNativeValue, resolveShape, ui, valueEditor, type DataSchema, type FormReferences } from "./native-value-form";
import { decodeNativeValue, type NativeValue } from "./native-values";
import { parseQueryParameters, type QueryPlan } from "./query-plan";
import { displayMessage, errorUiText, message, setUiAttr, setUiText, uiDisplayParameter } from "./ui-locale";

const emptySchema: DataSchema = { definitions: {}, fields: [], types: [], aliases: {} };

export function queryArgumentControls(
    draft: HTMLInputElement | HTMLTextAreaElement,
    readPlan: () => QueryPlan | null,
    readSchema: () => DataSchema | null,
    readReferences?: () => FormReferences,
    canApply: () => boolean = () => true,
): HTMLDetailsElement {
    const details = ui("details", "query-argument-controls");
    const summary = ui("summary", "", message("queryArguments.title"));
    const fields = ui("div");
    const status = ui("p", "data-help");
    status.setAttribute("role", "status");
    details.append(summary, fields, status);
    details.ontoggle = () => {
        fields.replaceChildren();
        setUiText(status, "");
        if (!details.open) return;

        try {
            const schema = readSchema() ?? emptySchema;
            const declarations = readPlan()?.parameters;
            if (!declarations?.length) {
                setUiText(status, message("queryArguments.noDeclarations"));
                return;
            }
            let lastDraft = draft.value;
            const signature = JSON.stringify(declarations);
            const existing = parseQueryParameters(lastDraft);
            const editors = declarations.map(declaration => {
                const { name, nullable, default: fallback } = declaration;
                const shape = resolveShape(declaration.shape, schema);
                const original = Object.hasOwn(existing, name) ? existing[name] : undefined;
                const hasDefault = fallback !== undefined;
                const group = ui("fieldset");
                const editor = ui("div");
                const mode = ui("select", "data-input");
                setUiAttr(mode, "aria-label", displayMessage("queryArguments.modeLabel", { p0: uiDisplayParameter(name) }));
                mode.required = true;
                if (!hasDefault) {
                    const missing = ui("option", "", message("queryArguments.chooseValue"));
                    missing.value = "";
                    mode.append(missing);
                }
                const valueOption = ui("option", "", message("queryArguments.enterValue"));
                valueOption.value = "value";
                mode.append(valueOption);
                if (hasDefault) {
                    const option = ui("option", "", message("queryArguments.useDefault"));
                    option.value = "default";
                    mode.append(option);
                }
                if (nullable) {
                    const option = ui("option", "", message("nativeValue.explicitNone"));
                    option.value = "null";
                    mode.append(option);
                }
                mode.value = original?.kind === "none" && nullable ? "null" : original ? "value" : hasDefault ? "default" : "";
                let value: NativeValue | undefined = original?.kind !== "none" ? original : undefined;
                const drawValue = () => {
                    if (mode.value !== "value") {
                        editor.hidden = true;
                        editor.replaceChildren();
                        return;
                    }
                    editor.hidden = false;
                    value ??= fallback?.kind !== "none" ? fallback : undefined;
                    value ??= emptyNativeValue(shape, schema);
                    editor.replaceChildren(valueEditor(shape, value, schema, next => { value = next; }, name, { parameterId: name, path: [] }, 0, readReferences?.()));
                };
                mode.onchange = () => {
                    const invalid = editor.querySelector<HTMLInputElement>(":invalid");
                    if (invalid) {
                        mode.value = "value";
                        invalid.reportValidity();
                        return;
                    }
                    drawValue();
                };
                group.append(ui("legend", "", name), mode);
                if (hasDefault)
                    group.append(ui("p", "data-help", displayMessage("queryArguments.defaultValue", { p0: uiDisplayParameter(formatNativeValue(fallback)) })));
                group.append(editor);
                drawValue();
                return {
                    name, group,
                    read(): NativeValue | undefined {
                        switch (mode.value) {
                            case "default": return undefined;
                            case "null": return { kind: "none" };
                            case "value": return value;
                        }
                    },
                };
            });
            fields.append(...editors.map(editor => editor.group), action(message("queryArguments.apply"), () => {
                if (!canApply()) return;
                try {
                    if (draft.value !== lastDraft || (readSchema() ?? emptySchema) !== schema || JSON.stringify(readPlan()?.parameters) !== signature) {
                        setUiText(status, message("queryArguments.changed"));
                        return;
                    }
                    const invalid = fields.querySelector<HTMLInputElement>(":invalid");
                    if (invalid) {
                        invalid.reportValidity();
                        return;
                    }
                    const next = new Map(Object.entries(existing));
                    for (const entry of editors) {
                        const value = entry.read();
                        if (value === undefined) next.delete(entry.name);
                        else {
                            if (!decodeNativeValue(value, 64)) {
                                setUiText(status, message("queryValidation.invalidNativeParameter"));
                                return;
                            }
                            next.set(entry.name, value);
                        }
                    }
                    lastDraft = JSON.stringify(Object.fromEntries(next), null, 2);
                    draft.value = lastDraft;
                    draft.dispatchEvent(new Event("input", { bubbles: true }));
                    setUiText(status, message("queryArguments.applied"));
                } catch (error) {
                    setUiText(status, error instanceof Error ? errorUiText(error) : String(error));
                }
            }));
        } catch (error) {
            setUiText(status, error instanceof Error ? errorUiText(error) : String(error));
        }
    };
    return details;
}
