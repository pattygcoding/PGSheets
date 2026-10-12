#!/usr/bin/env pwsh
<#
.SYNOPSIS
    The single entry point for this repository: `pg start`, `pg test`, `pg check`.

.DESCRIPTION
    Every real command runs from `rust/`, which is why this exists — it saves the `cd` and the crate
    flag. Invoke it from the repository root as `.\pg start` in PowerShell, or `pg start` in cmd, which
    also searches the current directory.

.PARAMETER Verb
    start    Build what changed and launch the app. The default.
    test     Run every test. Headless: no display, no window.
    check    Format, lint and test, the way CI does. Run this before pushing.
    release  Build the optimised binary.
    fmt      Format in place.

.EXAMPLE
    .\pg                      # same as: .\pg start
.EXAMPLE
    .\pg check
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('start', 'test', 'check', 'release', 'fmt')]
    [string] $Verb = 'start'
)

$ErrorActionPreference = 'Stop'

$rust = Join-Path $PSScriptRoot 'rust'

if (-not (Test-Path $rust)) {
    throw "There is no rust/ workspace next to this script ($PSScriptRoot); run it from the repository root."
}

Push-Location $rust

try {
    switch ($Verb) {
        'start' {
            # `cargo run` rebuilds whatever changed and then launches, which is what a start command
            # should do. The window is the process, so its exit code is the command's.
            cargo run -p pgsheets-app
        }

        'test' {
            cargo test --workspace
        }

        'check' {
            cargo fmt --all --check

            if ($LASTEXITCODE -ne 0) {
                throw 'Formatting is not clean; run `pg fmt` to fix it.'
            }

            cargo clippy --workspace --all-targets -- -D warnings

            if ($LASTEXITCODE -ne 0) {
                throw 'Clippy is not clean.'
            }

            cargo test --workspace
        }

        'release' {
            cargo build --workspace --release --locked

            if ($LASTEXITCODE -eq 0) {
                $binary = Join-Path $rust 'target\release\pgsheets.exe'
                Write-Host "`nBuilt. The binary is at $binary"
            }
        }

        'fmt' {
            cargo fmt --all
        }
    }

    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
finally {
    Pop-Location
}
