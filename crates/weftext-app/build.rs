use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn version_key(path: &Path) -> Option<Vec<u32>> {
    path.file_name()?
        .to_str()?
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

fn resource_compiler() -> PathBuf {
    if let Some(directory) = env::var_os("WEFTEXT_WINDOWS_SDK_BIN") {
        let executable = PathBuf::from(directory).join("rc.exe");
        assert!(executable.is_file(), "WEFTEXT_WINDOWS_SDK_BIN lacks rc.exe");
        return executable;
    }
    if let Some(directory) = env::var_os("WindowsSdkVerBinPath") {
        let directory = PathBuf::from(directory);
        for executable in [directory.join("rc.exe"), directory.join("x64/rc.exe")] {
            if executable.is_file() {
                return executable;
            }
        }
    }
    let base =
        PathBuf::from(env::var_os("ProgramFiles(x86)").expect("Windows SDK root unavailable"))
            .join("Windows Kits/10/bin");
    let mut versions: Vec<_> = fs::read_dir(base)
        .expect("Windows SDK is required for native executable resources")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter_map(|path| version_key(&path).map(|key| (key, path)))
        .collect();
    versions.sort_by(|left, right| right.0.cmp(&left.0));
    versions
        .into_iter()
        .map(|(_, path)| path.join("x64/rc.exe"))
        .find(|path| path.is_file())
        .expect("An installed x64 Windows SDK rc.exe is required")
}

fn quoted(path: &Path) -> String {
    path.to_str()
        .expect("Resource input path must be UTF-8")
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn main() {
    println!("cargo:rerun-if-changed=../../app/dist");
    println!("cargo:rerun-if-env-changed=WEFTEXT_WINDOWS_SDK_BIN");
    println!("cargo:rerun-if-env-changed=WindowsSdkVerBinPath");
    println!("cargo:rerun-if-env-changed=ProgramFiles(x86)");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    assert_eq!(env::var("CARGO_CFG_TARGET_ARCH").as_deref(), Ok("x86_64"));
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"))
        .join("../..");
    let packaging = root.join("packaging/windows");
    let icon = packaging.join("assets/weftext.ico");
    let manifest = packaging.join("application.manifest");
    for path in [&icon, &manifest, &packaging.join("host-contract.json")] {
        println!("cargo:rerun-if-changed={}", path.display());
        assert!(
            path.is_file(),
            "Missing native resource: {}",
            path.display()
        );
    }
    let version = env::var("CARGO_PKG_VERSION").expect("Cargo package version");
    let mut parts: Vec<u16> = ["MAJOR", "MINOR", "PATCH"]
        .map(|part| {
            env::var(format!("CARGO_PKG_VERSION_{part}"))
                .expect("Cargo version component")
                .parse()
                .expect("Windows version component must fit u16")
        })
        .to_vec();
    let pre = env::var("CARGO_PKG_VERSION_PRE").expect("Cargo prerelease component");
    let revision = if pre.is_empty() {
        u16::MAX
    } else {
        let value = pre
            .strip_prefix("rc.")
            .expect("Windows local versions support only rc.N")
            .parse::<u16>()
            .expect("rc.N must fit u16");
        assert!(value < u16::MAX, "stable reserves revision 65535");
        value
    };
    parts.push(revision);
    let numeric = parts
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let compiler = resource_compiler();
    for (binary, description) in [
        ("weftext", "Weftext desktop"),
        ("weftext-cli", "Weftext command line"),
    ] {
        let source = output.join(format!("{binary}.rc"));
        let resource = output.join(format!("{binary}.res"));
        let text = format!(
            "#pragma code_page(65001)\n1 ICON \"{}\"\n1 24 \"{}\"\n\
             1 VERSIONINFO\n FILEVERSION {numeric}\n PRODUCTVERSION {numeric}\n\
             FILEFLAGSMASK 0x3fL\n FILEFLAGS 0\n FILEOS 0x40004L\n FILETYPE 1\n FILESUBTYPE 0\n\
             BEGIN\n BLOCK \"StringFileInfo\"\n BEGIN\n BLOCK \"040904b0\"\n BEGIN\n\
             VALUE \"CompanyName\", \"Weftext\\0\"\n\
             VALUE \"FileDescription\", \"{description}\\0\"\n\
             VALUE \"FileVersion\", \"{version}\\0\"\n\
             VALUE \"InternalName\", \"{binary}\\0\"\n\
             VALUE \"OriginalFilename\", \"{binary}.exe\\0\"\n\
             VALUE \"ProductName\", \"Weftext\\0\"\n\
             VALUE \"ProductVersion\", \"{version}\\0\"\n\
             END\n END\n BLOCK \"VarFileInfo\"\n BEGIN\n\
             VALUE \"Translation\", 0x0409, 1200\n END\n END\n",
            quoted(&icon),
            quoted(&manifest),
        );
        fs::write(&source, text).expect("Cannot write owned resource input");
        let status = Command::new(&compiler)
            .args(["/nologo", "/r", "/fo"])
            .arg(&resource)
            .arg(&source)
            .status()
            .expect("Cannot start installed Windows resource compiler");
        assert!(
            status.success(),
            "Windows resource compiler rejected {binary}"
        );
        println!("cargo:rustc-link-arg-bin={binary}={}", resource.display());
    }
}
