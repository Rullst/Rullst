# Android signing and application icons

> Planned for **12.1.0**, not included in the published 12.0.0 CLI. Native
> compilation/signature verification in CI is not physical-device or store approval.

## Generate once, customize deliberately

Generate an Android shell using the [web-first tutorial](43-omni-web-first.md).
New shells use the locally embedded Rullst logo as a default, not a claim that
your application is an official Rullst product. Replace `omni-app/icons/icon.svg`
with your own square artwork before distribution when appropriate.

From `omni-app`, refresh **after** all Android/iOS initialization:

```bash
npm run tauri -- icon icons/icon.svg
```

Omni performs this final pass automatically for newly generated mobile shells.
Initialization failures and failed icon generation are errors. Existing shell
directories are not overwritten by `make:omni`.

## One-time signing identity

Android requires signed APKs. Keep the same application ID and signing identity
for updates; changing the key may prevent installation over an existing app.
A debug key is for development only. There is no shared Rullst production key.

Use the JDK's interactive `keytool` to create an application-owned key outside
the repository. The following path is illustrative; choose private storage and
restrict permissions to your account (on Windows, use an equivalent private ACL):

```bash
keytool -genkeypair -keystore /private/path/my-app-upload.jks \
  -storetype JKS -alias my-app-upload -keyalg RSA -keysize 3072 -validity 10000
```

Let `keytool` prompt for passwords; do not put them in command arguments, shell
history, chat, commits or logs. Back up the keystore and recovery information
securely. Key rotation, Play App Signing and upload-key recovery are separate
application/platform decisions. See [Android signing in Tauri](https://v2.tauri.app/distribute/sign/android/).

## Build a signed release APK

Supply these variables through your secret manager/CI or a private shell:

| Variable | Value |
| :--- | :--- |
| `RULLST_ANDROID_KEYSTORE` | Absolute path to your existing keystore |
| `RULLST_ANDROID_KEY_ALIAS` | Alias selected when creating the key |
| `RULLST_ANDROID_STORE_PASSWORD` | Keystore password |
| `RULLST_ANDROID_KEY_PASSWORD` | Private-key password |

Example for **Bash**, without storing passwords in history:

```bash
export RULLST_ANDROID_KEYSTORE=/private/path/my-app-upload.jks
export RULLST_ANDROID_KEY_ALIAS=my-app-upload
read -r -s -p 'Keystore password: ' RULLST_ANDROID_STORE_PASSWORD; printf '\n'
read -r -s -p 'Key password: ' RULLST_ANDROID_KEY_PASSWORD; printf '\n'
export RULLST_ANDROID_STORE_PASSWORD RULLST_ANDROID_KEY_PASSWORD
cargo rullst omni android --release
unset RULLST_ANDROID_STORE_PASSWORD RULLST_ANDROID_KEY_PASSWORD
```

Run this from the application root. It checks required inputs before invoking
the locally installed Tauri CLI. The generated Gradle release configuration
uses those inputs and rejects missing credentials even if you invoke Tauri
directly. Wrong passwords or aliases fail the native signing task. The command
does not launch a backend, upload an APK, publish to a store or modify secrets.

For a faster build targeting only ARM64 devices, run from `omni-app` with the
same environment: `npm run tauri -- android build --target aarch64 --apk --ci`.
This narrows supported device architectures; it is not a universal APK.

Do not share Gradle caches/build scans or run verbose/debug build logging with
production credentials. Native build tools inherit the signing environment;
run them only against reviewed application code on a trusted machine. Ignore
rules reduce accidental commits but do not protect already tracked files or
replace private key storage. Do not upload generated projects containing keys
as CI failure artifacts.

Before sharing an APK, use the Android SDK Build Tools:

```bash
apksigner verify --verbose --print-certs /path/to/app-release.apk
```

Check the signer against your expected certificate, install on a real device
and test upgrading an existing installation. `INSTALL_PARSE_FAILED_NO_CERTIFICATES`
means the APK lacks a usable signature, not that the user should disable Android
security. Do not distribute files named `*-unsigned.apk`. For a development
build only, use `npm run tauri -- android build --debug --apk --ci`.
[Android's apksigner reference](https://developer.android.com/tools/apksigner)
explains verification; APK signing does not establish Play Store acceptance.

## Existing 12.0.0 shells

Updating the CLI or framework dependency does **not** rewrite your generated
application. Commit/back up the existing shell and review the following changes:

1. Keep your application ID, version policy, native modifications and existing
   signing identity. Do not regenerate over the old directory.
2. Replace the icon source if desired and run the icon command after mobile init.
3. Review the [signing template](https://github.com/Rullst/Rullst/blob/main/cargo-rullst/src/generators/desktop/signing.gradle.kts)
   and append it to `omni-app/gen/android/app/build.gradle.kts` **only if you have
   no existing release-signing setup**. Do not duplicate/override another policy.
   Custom product flavors need corresponding validation-task wiring.
4. Ignore `*.jks`, `*.keystore`, `*.p12`, `keystore.properties` and `.gradle/`;
   keep real keys outside the repository and supply the four environment inputs.
5. Verify the resulting certificate and test an actual device update. Projects
   with an existing signing setup may retain their native Tauri build command;
   the convenience CLI checks for the Rullst guard instead of silently changing it.

The repository tests process ordering and failure propagation with controlled
tool fixtures. The hosted Android job separately builds a real release with a
disposable key and verifies its certificate. Those are distinct evidence levels.
