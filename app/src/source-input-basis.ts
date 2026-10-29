/** One client-side author input for source reads and edit proposals. This is
 * neither a Core Draft acknowledgement nor save, close or authorization proof. */
export interface SourceInputBasis {
    inputId: string;
    sessionId: string;
    source: string;
    serial: number;
    baseId: string;
    path: string;
    root: string;
    nodeId?: string;
    workspaceId?: string;
}

/** Deliberately copy only source identity, so no transport authority can leak
 * into a retained read/proposal when the complete document basis grows. */
export function sourceInputBasis(basis: SourceInputBasis): SourceInputBasis {
    const { inputId, sessionId, source, serial, baseId, path, root, workspaceId, nodeId } = basis;
    return { inputId, sessionId, source, serial, baseId, path, root, workspaceId, nodeId };
}

/** Same bytes after a different input or Undo/Redo do not revive an old read. */
export function sameSourceInput(current: SourceInputBasis | null, original: SourceInputBasis): boolean {
    return !!current && current.inputId === original.inputId && current.sessionId === original.sessionId
        && current.source === original.source && current.serial === original.serial && current.baseId === original.baseId
        && current.path === original.path && current.root === original.root
        && current.workspaceId === original.workspaceId && current.nodeId === original.nodeId;
}

export function sourceInputKey(basis: SourceInputBasis): string {
    return JSON.stringify([basis.inputId, basis.sessionId, basis.source, basis.serial, basis.baseId,
        basis.path, basis.root, basis.workspaceId, basis.nodeId]);
}
