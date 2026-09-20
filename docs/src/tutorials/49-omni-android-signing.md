# Android signing and application icons

> Application-owned signing shipped in **12.1.0**. The current **v13 development
> source** additionally requires CLI artifact/certificate verification as
> described below; its hosted acceptance is pending. Native compilation and
> signature verification do not establish physical-device or store approval.

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
| `RULLST_ANDROID_SIGNING_CERTIFICATE` (v13) | Absolute public DER certificate exported from the intended signing identity |
| `RULLST_ANDROID_APKSIGNER_JAR` (v13) | Absolute trusted SDK Build Tools `lib/apksigner.jar` |

For v13, export the public certificate once using the selected alias and
keystore. With the signing environment already set, run:

```bash
keytool -exportcert -keystore "$RULLST_ANDROID_KEYSTORE" \
  -alias "$RULLST_ANDROID_KEY_ALIAS" \
  -storepass:env RULLST_ANDROID_STORE_PASSWORD \
  -file /private/path/my-app-certificate.der
export RULLST_ANDROID_SIGNING_CERTIFICATE=/private/path/my-app-certificate.der
export RULLST_ANDROID_APKSIGNER_JAR=/path/to/reviewed-sdk/build-tools/VERSION/lib/apksigner.jar
```

Replace the illustrative paths and SDK version with your reviewed installation.
The certificate is public; keep its expected identity under application change
control. Java must be installed on an absolute trusted `PATH` entry. The
`--signing-certificate` and `--apksigner-jar` options override their respective
environment paths. Verification does not infer the expected certificate from
the APK it is checking.

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

Run this from the application root (on v13, set the two verification paths
above as well). It checks required inputs before invoking the locally installed
Tauri CLI. The generated Gradle release configuration
uses those inputs and rejects missing credentials even if you invoke Tauri
directly. Wrong passwords or aliases fail the native signing task. The command
does not launch a backend, upload an APK, publish to a store or modify secrets.

For a v13 ARM64 build, use `cargo rullst omni android --release --android-arch
aarch64`. On 12.1, run from `omni-app` with the same signing environment:
`npm run tauri -- android build --target aarch64 --apk --ci`. Restricting native
architectures also restricts the devices that can run that artifact.

## v13 output and certificate verification

The CLI inventories release APKs below
`omni-app/gen/android/app/build/outputs/apk` before and after the build. It
requires one fresh output; `--apk arm64/release/app-arm64-release.apk` is an
illustrative explicit relative selection when the build creates multiple
variants. Use the actual output path of your generated shell. Parent traversal,
linked outputs, absent/unchanged files and ambiguous fresh selections fail.
For a rebuild where Gradle would reuse an unchanged cached APK, move that
previous APK aside before rebuilding; the CLI does not delete it for you.
Filesystem timestamps must reflect writes during this invocation.

Discovery is bounded to 4,096 entries, depth eight, at most sixteen matching
APKs and 512 MiB of matching APK bytes. Each APK is nonempty and at most 512 MiB.
The expected certificate is at most 64 KiB. The CLI copies the selected APK to
a private temporary location, runs the trusted JDK/SDK verifier against those
exact bytes with SDK warnings treated as errors, requires one signer and matches the signing certificate's SHA-256.
It rechecks the original and snapshot before emitting a
`rullst.android-release.v1` JSON receipt with the path, length, APK digest and
certificate digest. A file changed afterwards is not covered by that receipt.

The verifier receives none of the four signing environment inputs. Captured
tool output is bounded and withheld on failure; errors do not replay build logs
that might contain credentials. The native build has a 45-minute limit and the
signature verifier a 90-second limit. Cancellation/errors perform best-effort
owned-process cleanup and remove the temporary snapshot. Trusted build tools,
SDK/JDK installation, filesystem custody and protection from other same-user
processes remain operator responsibilities. Multiple signers, signing-key
rotation lineages, AABs and custom output layouts need separate reviewed flows.

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
   The v2 guard covers Tauri's ABI-flavored `pre*ReleaseBuild` tasks. If you
   installed the unreleased v1 guard, review that task predicate and update its
   marker to v2; the CLI refuses to overwrite an application-owned v1 block.
   Custom build types other than `release` need corresponding validation wiring.
4. Ignore `*.jks`, `*.keystore`, `*.p12`, `keystore.properties` and `.gradle/`;
   keep real keys outside the repository and supply the four environment inputs.
5. Verify the resulting certificate and test an actual device update. Projects
   with an existing signing setup may retain their native Tauri build command;
   the convenience CLI checks for the Rullst guard instead of silently changing it.

The repository tests process ordering, stale/ambiguous output, wrong certificates,
byte changes, time/output bounds and failure redaction with controlled tools.
The hosted Android job is configured to build a real release through this CLI
with a disposable key, check its receipt and independently verify the signer
again through the SDK wrapper. Its v13 result remains required before admission;
local protocol fixtures do not establish SDK interoperability.
