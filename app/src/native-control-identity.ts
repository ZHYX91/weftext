import type { AuthorInputSession, ValuePath } from "./author-input";

/** Actual schema identity and Core path. Display labels never address author input. */
export type NativeControlIdentity = (
    | { fieldId: string; parameterId?: never }
    | { parameterId: string; fieldId?: never }
) & {
    path: readonly ValuePath[];
    /** Union alternatives distinguish UI controls without inventing a Core path. */
    variants?: readonly number[];
};

export interface NativeControlBinding { session: AuthorInputSession; slot: string }
export interface NativeControlAddress { readonly identity: NativeControlIdentity; readonly role: string; readonly key: string }
const addresses = new WeakMap<HTMLElement, NativeControlAddress>();

export function nativeControlKey(identity: NativeControlIdentity, role: string): string {
    const owner = identity.fieldId !== undefined ? identity.fieldId : ["queryParameter", identity.parameterId];
    return JSON.stringify([owner, identity.path, [role, identity.variants ?? []]]);
}

/** Register an actual control once; consumers use this address rather than decoding a display attribute. */
export function registerNativeControl(control: HTMLElement, identity: NativeControlIdentity, role: string): NativeControlAddress {
    const captured = Object.freeze({ ...identity, path: Object.freeze(identity.path.map(part => Object.freeze({ ...part }))), variants: identity.variants && Object.freeze([...identity.variants]) });
    const address = Object.freeze({ identity: captured, role, key: nativeControlKey(captured, role) });
    control.dataset.controlKey = address.key;
    addresses.set(control, address);
    return address;
}

export function nativeControlAddress(control: HTMLElement): NativeControlAddress {
    const address = addresses.get(control);
    if (!address || control.dataset.controlKey !== address.key) throw new Error("Native form control identity is missing or changed");
    return address;
}
