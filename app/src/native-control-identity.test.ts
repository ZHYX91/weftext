import test from "node:test";
import assert from "node:assert/strict";
import { nativeControlAddress, nativeControlKey, registerNativeControl, type NativeControlIdentity } from "./native-control-identity";
import { locale } from "./ui-locale";

test("control addresses retain exact FieldId/Core path across language changes", () => {
    const identity: NativeControlIdentity = { fieldId: "tasks/reminder", path: [{ kind: "member", key: "作者键\ud800" }, { kind: "index", index: 2 }], variants: [1] };
    locale.set("zh-CN"); const before = nativeControlKey(identity, "minute-helper"); locale.set("en-US");
    assert.equal(nativeControlKey(identity, "minute-helper"), before);
    assert.deepEqual(JSON.parse(before), [identity.fieldId, identity.path, ["minute-helper", [1]]]);
    assert.notEqual(nativeControlKey(identity, "exact-value"), before);
    assert.notEqual(nativeControlKey({ ...identity, variants: [0] }, "minute-helper"), before);
    assert.notEqual(nativeControlKey({ ...identity, path: [] }, "minute-helper"), before);
    locale.set("zh-CN");
});

test("a query parameter cannot alias a field or a second parameter", () => {
    const field = nativeControlKey({ fieldId: "same", path: [] }, "value"), parameter = nativeControlKey({ parameterId: "same", path: [] }, "value");
    assert.notEqual(field, parameter); assert.notEqual(parameter, nativeControlKey({ parameterId: "other", path: [] }, "value"));
    // @ts-expect-error Exactly one actual owner is required.
    const ambiguous: NativeControlIdentity = { fieldId: "field", parameterId: "parameter", path: [] };
    void ambiguous;
});

test("a registered control exposes its captured address independently of its label", () => {
    const path = [{ kind: "member" as const, key: "at" }];
    const control = { dataset: {} } as HTMLElement;
    const address = registerNativeControl(control, { fieldId: "tasks/reminder", path }, "minute-helper");
    path[0].key = "another";
    assert.equal(nativeControlAddress(control), address);
    assert.deepEqual(address.identity.path, [{ kind: "member", key: "at" }]);
    control.dataset.controlKey = "changed";
    assert.throws(() => nativeControlAddress(control), /identity/);
    assert.throws(() => nativeControlAddress({ dataset: {} } as HTMLElement), /identity/);
});
