Weftext Windows local candidate

weftext.exe is the desktop entry. weftext-cli.exe preserves real terminal input, output and exit codes. They use the same private state and Core; a repeated launch must not create another writer. Server and internal compiler workers do not initialize the desktop.

Starting requires the Microsoft WebView2 and VC Runtimes qualified for this candidate. prerequisites/ contains the qualified original offline installers and original licenses; neither the application nor the packager runs them automatically. Missing components are reported. Use and redistribution remain governed by their original licenses. Office is not included. Recognition still requires explicit separate setup using scripts/recognition/README.md in source.zip; an installed host runtime is not a bundled dependency.

Libraries remain in their chosen directories. Default personal durable state is %LOCALAPPDATA%/Weftext-MVP; cache and WebView2 user data are separate under %LOCALAPPDATA%/Weftext/Cache. Preserve durable state and library originals separately from disposable cache. The MSIX excludes only the exact durable subdirectory from virtualization. Actual installation, upgrade and uninstall preservation depend on acceptance of this same candidate.

Generation does not establish signature trust, installation or whole-version acceptance. An unsigned MSIX is not an install-ready trusted release; do not disable system protections to install it. BUILD.json and the outer PACKAGE.json identify source, both executables and materials. Equal candidate versions or filenames do not establish a successful upgrade. Do not overwrite previous candidates or replace tested bytes with another build.

See docs/MVP.md, docs/PACKAGING.md and docs/WORKFLOW.md in source.zip for usage, building and current support boundaries.
