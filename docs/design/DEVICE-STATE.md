---
source_language: zh-CN
translation_of: DEVICE-STATE.zh-CN.md
translation_status: synced
---

[简体中文](DEVICE-STATE.zh-CN.md)

# Local device state

This is the private Core foundation shared by the v0.7 native host, reminders and personal client settings. It defines no second note/task store, commit authority or synchronization identity, and establishes no completed native-host or reminder acceptance.

One Core `instance.lock` lease owns `<data-dir>/device-state`, retaining ParentGuard for its physical directory and ancestors. A synchronization `replica_id` belongs to a workspace/replica and cannot stand for this local profile's device identity.

## Sole identity and copying

`identity.json` is a strict version-1 record with a canonical UUIDv4, an explicit protection format and a bounded sealed binding. Windows uses noninteractive DPAPI `LOCAL_MACHINE | UI_FORBIDDEN` for `{version, id, user SID, original Core-directory FileIdentity}`. Qualification requires available Windows protection, the same user SID and the same physical directory volume/index/created identity. It is not an authentication credential or a promise that a whole-machine image clone becomes a globally distinct device.

`Core::device_identity` returns `{id, isCurrent}`. A new empty state generates it once. Ordinary process restart, upgrade or renaming the same physical profile retains it. A copied directory, another user or unavailable Windows protection is never silently adopted, overwritten or given another ID: it returns non-current or an evidence error while retaining originals. Missing identity with other device records or original pending files refuses new identity allocation. Explicit import/rebinding needs separate complete semantics preserving originals and reminder unknown states; startup cannot guess them.

## Separate records and CAS

The only canonical leaves are `identity.json`, `desktop-preferences.json`, `reminders.json` and `client-preferences.json`. Desktop intents are independent `resident`, `openLibraryOnLaunch` and `loginStartupRequested`, defaulting to false and persisted atomically by revision CAS. Login intent does not establish the actual OS StartupTask Enabled/DisabledByUser state. Retained foreign/unverified settings are inspectable, but their effective values are all off and updates are rejected. UI locale has a separate client-settings owner; reminder selection, opt-in and ledger have a reminder owner. Neither enters desktop intents or creates another device ID.

Bounded reads of the internal closed `DeviceRecordKind` return original bytes and lowercase SHA256. CAS None creates only an absent record; Some replaces only matching original bytes. This is CAS for serialized `&mut` consumers under one Core lease, not an atomic filesystem compare-and-swap against arbitrary external edits by the same OS user. A guarded new temporary is written, flushed and closed before same-volume WRITE_THROUGH publication and complete strict readback. Identity/preferences validate their schema before writing; other consumers must also perform their own strict validation first. Failure, differing readback and unknown effects do not trigger retries or rollback; originals and temporaries remain. A temporary name must be `.<known-leaf>.<canonical-UUIDv4>.pending`, not an arbitrary suffix whitelist. Bounds are 64 KiB identity, 16 KiB desktop/client settings each, 16 MiB reminders, and 128 directory members; exceeding them fails explicitly.

## Library backups and validation

Library owner backup qualifies the device root, known leaves/temporary names, original objects and strict versioned schemas, fixes a routing witness and retains the same handles. Device-global payloads never become library-author CapturedFiles or backup contents. Unknown members, directories/aliases, bad schemas and records without a linked real strict validator are rejected; the private root is not broadly ignored. Reminder/client owners must connect their real validators at the sole classification entry rather than treating `{}` as successful ignored state.

The witness fixes only the physical device root and `deviceGlobal:v1` classification, excluding legitimate record membership, payload and stamps. Every capture/execution still revalidates strictly. Legitimate reminder-ledger/preference updates do not invalidate an unrelated author-library backup, while an unknown added member still blocks execution.

Meaningful validation covers reopening one profile, independent settings, stale CAS preserving bytes, non-adoption of physical copies, retained invalid/unknown/oversize originals, directory guards and backup exclusion/rejection. Source and controlled tests do not replace actual upgrade/uninstall, protection across machines, OS setting behavior, full network isolation or power-loss acceptance.

Platform references: [DPAPI protect](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata), [DPAPI unprotect](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata).
