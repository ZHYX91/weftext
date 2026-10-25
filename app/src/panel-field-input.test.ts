import test from "node:test";
import assert from "node:assert/strict";
import { createPanelFieldInputs } from "./panel-field-input";
import type { AuthorInputOwner } from "./author-input";
import type { DocumentBasis } from "./data-panel";

const original: DocumentBasis = { inputId: "test-input", sessionId:"session-A", source:"原文", serial:3, baseId:"base", path:"display-path", root:"root" };

test("the field port binds the actual field identity and composes host and proposal lifetimes", () => {
    const basis = {...original}; let actualSession = original.sessionId, allowed = true, proposal = true;
    let received: {owner:AuthorInputOwner; current:()=>boolean; revision?:()=>unknown} | undefined;
    const revision = () => 7;
    const fieldInputs = createPanelFieldInputs((owner,current,_sink,leaseRevision) => { received = {owner,current,revision:leaseRevision}; return undefined; }, (captured, fieldId) => {
        assert.equal(fieldId,"tasks/reminder"); assert.notEqual(captured,basis); assert.ok(Object.isFrozen(captured));
        return {resource:"actual-resource-A", current:()=>allowed && actualSession===captured.sessionId};
    });
    fieldInputs({basis,fieldId:"tasks/reminder",current:()=>proposal,revision});
    assert.deepEqual(received!.owner,{kind:"field",resource:"actual-resource-A",fieldId:"tasks/reminder"});
    assert.equal(received!.revision,revision); assert.equal(received!.current(),true);
    basis.sessionId="session-B"; actualSession="session-B";
    assert.equal(received!.current(),false,"a mutable caller basis cannot reassign an old field producer");
    actualSession="session-A"; proposal=false; assert.equal(received!.current(),false);
    proposal=true; allowed=false; assert.equal(received!.current(),false);
});

test("an unavailable field owner or retired proposal never falls back to a body producer", () => {
    let calls=0, resolveCalls=0;
    const factory = createPanelFieldInputs(()=>{calls++;return undefined;},()=>{resolveCalls++;return undefined;});
    assert.equal(factory({basis:original,fieldId:"tasks/reminder",current:()=>true}),undefined);
    assert.equal(factory({basis:original,fieldId:"tasks/reminder",current:()=>false}),undefined);
    assert.equal(factory({basis:original,fieldId:"",current:()=>true}),undefined);
    assert.equal(calls,0); assert.equal(resolveCalls,1);
});
