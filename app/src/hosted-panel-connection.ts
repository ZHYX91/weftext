export type HostedPanelConnection = { label: string; connect(): void | Promise<void> };
export type HostedPanelFailure = { label: string; message: string };

/** Independent panels share an authorization lifetime, not a failure outcome. */
export async function connectHostedPanels(current: () => boolean, panels: HostedPanelConnection[]): Promise<HostedPanelFailure[] | null> {
  const failures: HostedPanelFailure[] = [];
  for (const panel of panels) {
    if (!current()) return null;
    try { await panel.connect(); }
    catch (error) {
      if (!current()) return null;
      failures.push({ label: panel.label, message: error instanceof Error ? error.message : "连接未完成" });
    }
    if (!current()) return null;
  }
  return failures;
}
