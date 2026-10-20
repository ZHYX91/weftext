import assert from 'node:assert/strict';

export async function beginTableCopyObservation(page, label) {
  await page.evaluate(label => {
    const surface = document.querySelector('.writing-table-surface'), tools = surface?.querySelector('.writing-table-tools');
    const button = [...surface?.querySelectorAll('button') ?? []].find(item => item.textContent === label), notice = surface?.querySelector('[role=status]');
    if (!surface || !tools || !button || !notice) throw new Error('Actual table copy controls are unavailable');
    const trace = { label, events: [], removals: 0 }, snapshot = kind => trace.events.push({ kind, at: performance.now(), connected: surface.isConnected && button.isConnected && notice.isConnected, busy: tools.getAttribute('aria-busy'), disabled: button.disabled, ariaDisabled: button.getAttribute('aria-disabled'), focused: document.activeElement === button, notice: notice.textContent });
    const observer = new MutationObserver(records => { for (const record of records) for (const removed of record.removedNodes) if (removed === surface || removed instanceof Element && removed.contains(surface)) trace.removals++; snapshot('mutation'); });
    observer.observe(document.body, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ['aria-busy', 'aria-disabled', 'disabled'] });
    snapshot('before-click'); window.__tableCopyObservation = { trace, observer, snapshot };
  }, label);
}

export async function endTableCopyObservation(page) {
  await page.locator('.writing-table-surface').first().locator('.writing-table-tools[aria-busy="false"]').waitFor({ timeout: 30000 });
  const trace = await page.evaluate(() => { const capture = window.__tableCopyObservation; if (!capture) throw new Error('Copy observer is missing'); capture.snapshot('settled'); capture.observer.disconnect(); delete window.__tableCopyObservation; return capture.trace; });
  assert.equal(trace.removals, 0, 'A copy/status-only command must not remove its original table surface');
  assert.ok(trace.events.every(event => event.connected), 'The original surface, command and notice must stay connected');
  const busy = trace.events.filter(event => event.busy === 'true');
  assert.ok(busy.length, 'Actual command busy state must be observed');
  assert.ok(busy.every(event => event.focused && !event.disabled && event.ariaDisabled === 'true'), 'Busy copy preserves original command focus and exposes its unavailable state');
  assert.deepEqual({ busy: trace.events.at(-1).busy, focused: trace.events.at(-1).focused, disabled: trace.events.at(-1).disabled, ariaDisabled: trace.events.at(-1).ariaDisabled }, { busy: 'false', focused: true, disabled: false, ariaDisabled: 'false' });
  return trace;
}
