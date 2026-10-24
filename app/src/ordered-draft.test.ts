import test from "node:test";
import assert from "node:assert/strict";
import { OrderedDraft } from "./ordered-draft";
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => resolve = done); return {promise, resolve}; }
const input = {sessionId:"writer", source:"S", serial:1, inputId:"first-edit", baseId:"B"};
test("one writer persists S then Y in dispatch order while frozen S retains only its own ACK", async () => {
 const order = new OrderedDraft(), gate = deferred<{inputRevision:number}>(), sent: any[]=[];
 const send = async (wire: any) => { sent.push(wire); return wire.inputRevision === 1 ? gate.promise : {inputRevision:wire.inputRevision}; };
 const first = order.persist(input, send), later = {...input,source:"Y",serial:2,inputId:"second-edit"}, second = order.persist(later, send);
 await Promise.resolve(); await Promise.resolve(); assert.equal(sent.length,1); assert.equal(order.acknowledged(input),undefined);
 gate.resolve({inputRevision:1}); assert.equal(await first,1); assert.equal(await second,2);
 assert.deepEqual(sent.map(wire=>[wire.source,wire.inputRevision]),[["S",1],["Y",2]]);
 assert.equal(order.acknowledged(input),undefined); assert.equal(order.acknowledged(later),2);
});
test("equal-byte Undo, pending literal and a new writer cannot borrow an older ACK", async () => {
 const order = new OrderedDraft(); await order.persist(input,async wire=>({inputRevision:wire.inputRevision}));
 for (const changed of [{inputId:"undo-back-to-S"},{serial:2},{sessionId:"other-writer"}]) assert.equal(order.acknowledged({...input,...changed}),undefined);
 const next = new OrderedDraft(); assert.equal(next.acknowledged(input),undefined);
});
test("a failed ACK never becomes current and the next actual input can still persist",async()=>{
 const order=new OrderedDraft(); await assert.rejects(order.persist(input,async()=>({inputRevision:99}))); assert.equal(order.acknowledged(input),undefined);
 assert.equal(await order.persist({...input,retainedBaseSource:"original B",baseId:undefined},async wire=>{assert.equal(wire.retainedBaseSource,"original B");return {inputRevision:wire.inputRevision};}),2);
});
