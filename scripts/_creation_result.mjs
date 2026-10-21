// Test-only projection; raw transports and their evidence retain the full envelope.
import assert from 'node:assert/strict';
export function createdDocument(envelope, operationId) {
 assert.equal(envelope.status, 'committed');
 assert.equal(typeof envelope.operationId, 'string');
 if(operationId!==undefined)assert.equal(envelope.operationId,operationId);
 assert.equal(envelope.documentError, undefined);
 assert.equal(typeof envelope.document?.sessionId, 'string');
 return envelope.document;
}
