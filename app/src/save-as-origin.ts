/** The OS path picker cannot redirect an old SaveAs intent to a new tab. */
export async function originalSaveAsSelection<T>(
    owner: T, sessionId: string,
    select: () => Promise<string | null>,
    current: () => { owner: T | null; session: string | null; active: boolean },
): Promise<{ owner: T; path: string } | null> {
    const path = await select();
    if (!path) return null;
    const now = current();
    if (!now.active || now.owner !== owner || now.session !== sessionId)
        throw new Error("saveAsOriginChanged");
    return { owner, path };
}
