---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Core repair probes

These bounded experiments use Python's standard library. They are evidence for specific candidate algorithms, not a production backend or acceptance suite.

Keep these files together:

- `namespace_probe.py` — namespace/creation/deletion experiment.
- `win32_probe.py` — its complete local ctypes helper and child-process entry point; no other dependency.
- `logical_cut_model.py` — independent in-memory common-cut model.
- `namespace-stdout.json`, `logical-cut-stdout.json` — full passing outputs from this bundle.
- `MANIFEST.json` — SHA256 of the scripts and outputs.

From this directory, on Windows with Python 3.9 or newer:

```powershell
python namespace_probe.py > namespace-stdout.json
python logical_cut_model.py > logical-cut-stdout.json
```

The namespace experiment creates a fresh `namespace-run-*` directory beside the scripts and retains it. It never recursively cleans up. Deletion tests select only their own known file and actual empty directory; an inserted unknown file must survive. Output records a relative fixture name. The model performs no filesystem operations and runs on other Python platforms too.

The recorded namespace run at `2026-10-12T13:44:57.094972+00:00` used local Windows/NTFS. LIST_DIRECTORY/READ_ATTRIBUTES directory handles sharing READ|WRITE without DELETE rejected external ancestor and target-parent renames (error32), while the selected directory moved through its own handle. The guards did **not** exclude a new sibling. Earlier attribute-only handles had failed to protect ancestor placement; this stronger access is part of the tested candidate.

`NtCreateFile` uses a held parent handle and one relative name, FILE_CREATE and FILE_DIRECTORY_FILE, returning the original LIST/ATTR/DELETE creation handle with FILE_CREATED. A second same-name creation failed with status `0xc0000035`; the creation handle remained held and external copy-root rename failed32. The test does not infer ownership from mkdir followed by reopening. Fixture setup still uses ordinary mkdir; its fixture setup is not the operation under test.

Created planned file handles (RW share0) rejected external writes32. Unknown target children remained possible and preserved. Selected file disposition succeeded; directory disposition failed145 when an unknown child made the directory nonempty, preserving that child. An explicitly empty created directory was removed. This supports fixed copy membership and delete-if-empty, not an exclusive subtree snapshot or all-child CAS. The Win32 absolute same-handle rename succeeded in this run; the native-relative-rename fallback was not exercised.

The model reproduces individually read A-old10/B-new10 sum20 across one transaction. A common retained cut returns sum10; frozen pages remain stable after live reordering, and a modeled authorization change invalidates delivery.

Not established: production ancestry-chain/alias/reparse validation, all handle/mapping/filter modes, nested closure capacity/upgrades, multi-step Node rename recovery, interruption/power-loss or metadata durability, whole-workspace external atomic observations, Server security/fencing, production retention/publication or performance. Passing this bundle does not close independent review findings. Changes to files outside the fixture remain outside its threat/test model.

The namespace bundle is a self-contained extraction of earlier bounded experiments, with a new directory-creation-handle case and a fresh full run. It does not alter the earlier scripts or results. API mechanisms: [NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile), [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew), [FILE_RENAME_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info), [SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle), [FILE_DISPOSITION_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_disposition_info).
