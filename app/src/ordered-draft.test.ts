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

test("retirement retries the original immutable transmission without lowering a newer ACK",async()=>{
 const order=new OrderedDraft();let original!:Parameters<OrderedDraft["retry"]>[0];const sent:any[]=[];
 await assert.rejects(order.persist(input,async wire=>{sent.push(wire);throw new Error("reply lost");},cut=>original=cut));
 const newer={...input,source:"Y",serial:2,inputId:"real-new-input"};
 await order.persist(newer,async wire=>{sent.push(wire);return {inputRevision:wire.inputRevision};});
 await order.retry(original,async wire=>{sent.push(wire);return {inputRevision:wire.inputRevision};});
 assert.deepEqual(sent.map(wire=>[wire.source,wire.inputRevision]),[["S",1],["Y",2],["S",1]]);
 assert.equal(order.acknowledged(newer),2);assert.equal(order.acknowledged(input),undefined);
 assert.equal(order.acknowledged({...newer,baseId:"different-base"}),undefined);
 await assert.rejects(new OrderedDraft().retry(original,async()=>{throw new Error("foreign cut must not send");}));
 assert.equal(await order.persist({...newer,source:"Z",inputId:"next-edit"},async wire=>({inputRevision:wire.inputRevision})),3);
});

test("a consumed original cut stays rejected instead of being issued under a new revision",async()=>{
 const order=new OrderedDraft();let cut!:Parameters<OrderedDraft["retry"]>[0];
 await order.persist(input,async wire=>({inputRevision:wire.inputRevision}),value=>cut=value);
 let failed=0;const reject=async(wire:any)=>{failed++;assert.equal(wire.inputRevision,1);throw new Error("Core cut consumed");};
 await assert.rejects(order.retry(cut,reject));await assert.rejects(order.retry(cut,reject));assert.equal(failed,2);
 const rebased={...input,baseId:"actual-committed-base"};
 assert.equal(await order.persist(rebased,async wire=>({inputRevision:wire.inputRevision})),2);
});
