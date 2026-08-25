import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import React from "react";
import ReactDOM from "react-dom/client";
import { WeftextApp } from "../../../prototypes/webui/app/page";
import "../../../prototypes/webui/app/globals.css";
import "./desktop.css";

declare global {
  interface Window {
    weftextDesktop?: {
      request(path: string, body?: unknown): Promise<unknown>;
      restoreWorkspace(): Promise<unknown>;
      chooseWorkspace(): Promise<unknown | null>;
      chooseMarkdownExportDestination(suggestedName: string): Promise<{ capability: string; displayPath: string } | null>;
      chooseTaskImportReceiptDestination(suggestedName: string): Promise<{ capability: string; displayPath: string } | null>;
      chooseBackupDirectory?(kind: "backup_parent" | "snapshot" | "restore_parent" | "drill_parent" | "drill_results_parent"): Promise<{ capability: string; kind: "backup_parent" | "snapshot" | "restore_parent" | "drill_parent" | "drill_results_parent"; displayPath: string } | null>;
    };
  }
}

window.weftextDesktop = {
  request(path, body) {
    return invoke("desktop_request", { path, body: body ?? null });
  },
  restoreWorkspace() {
    return invoke("restore_workspace");
  },
  async chooseWorkspace() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择 Weftext 工作区",
    });
    if (!selected || Array.isArray(selected)) return null;
    return invoke("open_workspace", { path: selected });
  },
  async chooseMarkdownExportDestination(suggestedName) {
    return invoke<{ capability: string; displayPath: string } | null>("choose_markdown_export_destination", { suggestedName });
  },
  async chooseTaskImportReceiptDestination(suggestedName) {
    return invoke<{ capability: string; displayPath: string } | null>("choose_task_import_receipt_destination", { suggestedName });
  },
  async chooseBackupDirectory(kind) {
    return invoke<{ capability: string; kind: "backup_parent" | "snapshot" | "restore_parent" | "drill_parent" | "drill_results_parent"; displayPath: string } | null>("choose_backup_directory", { kind });
  },
};

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <WeftextApp demo={null} />
  </React.StrictMode>,
);
