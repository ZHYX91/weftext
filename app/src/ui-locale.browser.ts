/** Component-check entry keeps one dependency graph even when Vite has recent HMR versions. */
export * as localeApi from "./ui-locale";
export { ui, valueEditor, nativeFormControls, nativeControlAddress, nativeControlKey } from "./data-panel";
export { graphCanvas } from "./knowledge-graph-canvas";
export { defaultCamera } from "./relationship-graph-state";
export { tagPicker } from "./tag-picker";
export { TagTree } from "./tag-tree";
export { openEverydayMenu } from "./everyday-ui";
export { InterfaceLocaleControl } from "./ui-locale-settings";
export { SourceEditor } from "./editor";
export { localizedGotoLine } from "./source-search";
export { openSearchPanel, getSearchQuery } from "@codemirror/search";
export { undo, redo, undoDepth, redoDepth } from "@codemirror/commands";
export { RecognitionPanel } from "./recognition-panel";
export { ProofreadingPanel } from "./proofreading-panel";
export { defaultProcessingSettings } from "./processing-settings";
