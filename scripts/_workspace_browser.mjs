// Exercise the actual task tabs. This helper has no hidden-control fallback and
// scopes commands to the selected Ribbon panel, never a similarly named dialog.
const tabs = new Set(['file', 'home', 'insert', 'knowledge', 'review', 'view']);

export async function workspaceTab(page, tab) {
  if (!tabs.has(tab)) throw new Error('Unknown workspace task tab');
  const ribbon = page.locator('.workspace-ribbon');
  await ribbon.locator(`[role="tab"][data-tab="${tab}"]`).click();
  const panel = ribbon.locator(`[role="tabpanel"][data-tab="${tab}"]`);
  await panel.waitFor({ state: 'visible' });
  return panel;
}

export async function workspaceCommand(page, tab, name) {
  const panel = await workspaceTab(page, tab);
  const button = panel.getByRole('button', { name, exact: true });
  await button.click();
  return button;
}

export async function workspaceMode(page, name) {
  return workspaceCommand(page, 'view', name);
}
