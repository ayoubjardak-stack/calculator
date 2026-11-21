"""Builds an Android APK of the calculator.

Needs: Android SDK (build-tools + a platform) and NDK, Java (for keytool),
and the Rust targets:  rustup target add aarch64-linux-android x86_64-linux-android

Run from the project root:  python tools/build_apk.py
Output: target/apk/calculator.apk  (64-bit phones only, plus the x86_64 emulator)
"""
import os
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "target" / "apk"
MIN_SDK = 24       # Android 7.0 and newer
TARGET_SDK = 34
PACKAGE = "com.example.calculator"
LIB_NAME = "calculator_app"

# Rust target -> (Android ABI folder, NDK clang prefix)
TARGETS = {
    "aarch64-linux-android": ("arm64-v8a", "aarch64-linux-android"),  # 64-bit phones
    "x86_64-linux-android": ("x86_64", "x86_64-linux-android"),       # emulator on a PC
}


def find_sdk() -> Path:
    for var in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        if os.environ.get(var):
            return Path(os.environ[var])
    if sys.platform == "win32":
        return Path(os.environ["LOCALAPPDATA"]) / "Android" / "Sdk"
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Android" / "sdk"
    return Path.home() / "Android" / "Sdk"


def latest(folder: Path) -> Path:
    def key(p: Path):
        return [int(x) if x.isdigit() else 0 for x in p.name.replace("android-", "").split(".")]
    items = sorted((p for p in folder.iterdir() if p.is_dir()), key=key)
    if not items:
        sys.exit(f"nothing found in {folder}")
    return items[-1]


def find_keytool() -> str:
    """keytool is often not on PATH even when Java is installed."""
    found = shutil.which("keytool")
    if found:
        return found
    exe = "keytool.exe" if sys.platform == "win32" else "keytool"
    candidates = []
    if os.environ.get("JAVA_HOME"):
        candidates.append(Path(os.environ["JAVA_HOME"]) / "bin" / exe)
    if sys.platform == "win32":
        for base in (Path(r"C:\Program Files\Java"), Path(r"C:\Program Files\Eclipse Adoptium")):
            if base.exists():
                candidates += sorted(base.glob(f"*/bin/{exe}"), reverse=True)
        candidates.append(Path(r"C:\Program Files\Android\Android Studio\jbr\bin") / exe)
    for c in candidates:
        if c.exists():
            return str(c)
    sys.exit("keytool not found: install a JDK or set JAVA_HOME")


def run(cmd, **kw):
    print(">", " ".join(str(c) for c in cmd))
    subprocess.run([str(c) for c in cmd], check=True, **kw)


def main():
    sdk = find_sdk()
    ndk = Path(os.environ["ANDROID_NDK_HOME"]) if os.environ.get("ANDROID_NDK_HOME") else latest(sdk / "ndk")
    build_tools = latest(sdk / "build-tools")
    platform = sdk / "platforms" / f"android-{TARGET_SDK}"
    if not platform.exists():
        platform = latest(sdk / "platforms")
    android_jar = platform / "android.jar"

    host = {"win32": "windows-x86_64", "darwin": "darwin-x86_64"}.get(sys.platform, "linux-x86_64")
    llvm = ndk / "toolchains" / "llvm" / "prebuilt" / host / "bin"
    ext = ".cmd" if sys.platform == "win32" else ""
    exe = ".exe" if sys.platform == "win32" else ""
    bat = ".bat" if sys.platform == "win32" else ""

    print(f"SDK: {sdk}\nNDK: {ndk}\nbuild-tools: {build_tools.name}, platform: {platform.name}")

    # 1. Compile the Rust library for each ABI.
    env = dict(os.environ)
    for target, (_abi, clang_prefix) in TARGETS.items():
        t = target.upper().replace("-", "_")
        clang = llvm / f"{clang_prefix}{MIN_SDK}-clang{ext}"
        env[f"CARGO_TARGET_{t}_LINKER"] = str(clang)
        env[f"CC_{target.replace('-', '_')}"] = str(clang)
        env[f"AR_{target.replace('-', '_')}"] = str(llvm / f"llvm-ar{exe}")
        run(["cargo", "build", "--release", "--lib", "--target", target], cwd=ROOT, env=env)

    # 2. Resources (launcher icon) and manifest.
    if OUT.exists():
        shutil.rmtree(OUT)
    res = OUT / "res"
    densities = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}
    from PIL import Image
    icon = Image.open(ROOT / "assets" / "icon.png").convert("RGBA")
    for name, size in densities.items():
        d = res / f"mipmap-{name}"
        d.mkdir(parents=True)
        icon.resize((size, size), Image.LANCZOS).save(d / "ic_launcher.png")
    values = res / "values"
    values.mkdir(parents=True)
    (values / "strings.xml").write_text(
        '<resources><string name="app_name">Calculator</string></resources>', encoding="utf-8")

    manifest = OUT / "AndroidManifest.xml"
    manifest.write_text(f"""<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{PACKAGE}" android:versionCode="1" android:versionName="1.0">
    <uses-sdk android:minSdkVersion="{MIN_SDK}" android:targetSdkVersion="{TARGET_SDK}" />
    <!-- No GPU requirement: iced falls back to software drawing (tiny-skia)
         when the phone has no usable Vulkan / OpenGL ES 3. -->
    <supports-screens android:smallScreens="true" android:normalScreens="true"
        android:largeScreens="true" android:xlargeScreens="true" android:anyDensity="true" />
    <application android:label="@string/app_name" android:icon="@mipmap/ic_launcher"
        android:hasCode="false" android:extractNativeLibs="true"
        android:theme="@android:style/Theme.DeviceDefault.NoActionBar">
        <activity android:name="android.app.NativeActivity" android:exported="true"
            android:configChanges="orientation|screenSize|smallestScreenSize|screenLayout|keyboardHidden|keyboard|navigation|uiMode|density|fontScale|locale|layoutDirection"
            android:resizeableActivity="true"
            android:launchMode="singleTask">
            <meta-data android:name="android.app.lib_name" android:value="{LIB_NAME}" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
""", encoding="utf-8")

    # 3. Link resources into a base APK.
    aapt2 = build_tools / f"aapt2{exe}"
    compiled = OUT / "compiled.zip"
    run([aapt2, "compile", "--dir", res, "-o", compiled])
    unsigned = OUT / "unsigned.apk"
    run([aapt2, "link", "-o", unsigned, "-I", android_jar, "--manifest", manifest, compiled])

    # 4. Add the native libraries.
    with zipfile.ZipFile(unsigned, "a", zipfile.ZIP_DEFLATED) as apk:
        for target, (abi, _clang_prefix) in TARGETS.items():
            so = ROOT / "target" / target / "release" / f"lib{LIB_NAME}.so"
            apk.write(so, f"lib/{abi}/lib{LIB_NAME}.so")

    # 5. Align and sign with a debug key (fine for installing on your own devices).
    aligned = OUT / "aligned.apk"
    run([build_tools / f"zipalign{exe}", "-f", "-p", "4", unsigned, aligned])
    keystore = Path.home() / ".android" / "debug.keystore"
    if not keystore.exists():
        keystore.parent.mkdir(parents=True, exist_ok=True)
        run([find_keytool(), "-genkeypair", "-keystore", keystore, "-storepass", "android",
             "-alias", "androiddebugkey", "-keypass", "android", "-keyalg", "RSA",
             "-keysize", "2048", "-validity", "10000", "-dname", "CN=Android Debug,O=Android,C=US"])
    final = OUT / "calculator.apk"
    run([build_tools / f"apksigner{bat}", "sign", "--ks", keystore, "--ks-pass", "pass:android",
         "--ks-key-alias", "androiddebugkey", "--key-pass", "pass:android", "--out", final, aligned])
    print(f"\nAPK ready: {final}  ({final.stat().st_size / 1e6:.1f} MB)")


if __name__ == "__main__":
    main()
