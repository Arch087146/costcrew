# winget manifests

These three files submit costcrew to the official winget community repo.

## How to submit (one-time per release)

1. Fork https://github.com/microsoft/winget-pkgs
2. Copy the `0.4.0/` folder (all three YAML files) to
   `manifests/a/Arch087146/costcrew/0.4.0/` in your fork.
3. Open a PR against `microsoft/winget-pkgs` with title
   `Add: Arch087146.costcrew version 0.4.0`.
4. The PR checks will run winget's manifest validator automatically.
   Once a maintainer approves, the package goes live and
   `winget install Arch087146.costcrew` works.

## Updating hashes

`InstallerSha256` in `Arch087146.costcrew.installer.yaml` is the SHA256 of
`costcrew-windows-x86_64.zip` from release v0.4.0. When a new version is
released, compute it with:

```powershell
Get-FileHash .\costcrew-windows-x86_64.zip -Algorithm SHA256
```

## Validating locally (optional)

```sh
winget install Microsoft.Winget.SourceValidation
winget validate --manifest .\0.4.0\
```
