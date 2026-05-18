# Vendor Tooling Manual Setup

`af` does not automatically install vendor EDA tools. Vendor installers,
licenses, EULAs, USB drivers, and account downloads are outside the lifecycle
that `af` can safely own. The CLI therefore treats tools such as `gw_sh` and
`programmer_cli` as detect-only/manual dependencies.

Use this runbook when `af tooling check --profile vendor --json` reports
missing Gowin tools.

## Policy

- Download vendor installers only from the official vendor account or portal.
- Do not commit installers, extracted vendor trees, license files, USB rules, or
  machine-bound activation artifacts.
- Prefer a user-controlled install prefix such as `$HOME/eda/gowin/<version>` or
  an admin-managed prefix such as `/opt/gowin/<version>`.
- Keep the base `af` Docker runtime OSS-only. Use bind mounts for private vendor
  installations instead of copying vendor tools into a redistributable image.
- Keep a local record of the vendor tool version, installer filename, checksum,
  install prefix, accepted license/EULA date, and board runner that uses it.

## Host Install

1. Download the Gowin EDA installer manually and verify it according to the
   vendor instructions. Record the checksum before executing it.

2. Install into an explicit prefix. Replace the placeholder path with the
   versioned directory you chose:

   ```bash
   export GOWIN_HOME="$HOME/eda/gowin/<version>"
   mkdir -p "$GOWIN_HOME"
   ```

   Run the vendor installer manually and select `$GOWIN_HOME` as the target
   directory. Use the installer UI or vendor-documented silent mode; do not rely
   on `af` to pass installer-specific shell flags.

3. Locate the CLI tools after installation:

   ```bash
   find "$GOWIN_HOME" -type f \( -name gw_sh -o -name programmer_cli \) -print
   ```

4. Add the directories that contain those binaries to your shell profile, or
   keep them in a project-local runner script. Example:

   ```bash
   export GOWIN_HOME="$HOME/eda/gowin/<version>"
   export PATH="$GOWIN_HOME/IDE/bin:$GOWIN_HOME/Programmer/bin:$PATH"
   ```

   Adjust the two PATH entries to match the actual directories discovered by
   `find`; vendor layouts can differ by release and OS.

5. If your Gowin edition requires license configuration, set the vendor-required
   license environment variables outside the repository. Do not commit license
   paths or server details.

6. Validate host visibility:

   ```bash
   gw_sh --version
   programmer_cli --version
   af tooling check --profile vendor --json
   af doctor --json
   ```

## Private Docker Usage

Do not add Gowin tools to the default `Dockerfile`. For private hardware
runners, bind-mount an already installed vendor tree read-only:

```bash
docker run --rm \
  -v "$PWD:/work" \
  -v "$GOWIN_HOME:/opt/gowin:ro" \
  -w /work \
  -e GOWIN_HOME=/opt/gowin \
  -e PATH="/opt/gowin/IDE/bin:/opt/gowin/Programmer/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
  accelfury-af:oss \
  cargo run -p af-cli --bin af -- tooling check --profile vendor --json
```

For programming hardware from a container, add only the required USB/JTAG device
passthrough and udev permissions for that runner. Avoid `--privileged` unless a
controlled local hardware lab policy explicitly permits it.

## Troubleshooting

- `gw_sh: command not found`: verify `PATH` and the binary location discovered
  under `$GOWIN_HOME`.
- `programmer_cli: command not found`: install or enable the vendor programmer
  component, then add its binary directory to `PATH`.
- Shared library errors: install only the OS libraries documented by the vendor
  for your platform; keep the list in local runner notes.
- License failures: verify vendor license environment variables or activation
  state outside the repository.
- USB/JTAG permission failures: install vendor udev rules or grant the runner
  user access to the exact USB device/group required by the board.
- Docker cannot see the programmer: pass through the specific USB device and
  confirm `programmer_cli --version` works inside the container before flashing.
