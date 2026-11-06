param(
    [string]$Toolchain = '1.92.0',
    [switch]$Offline,
    [switch]$RunNative
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$output = Join-Path $repo "target/slint-consumer/$PID"
$packages = Join-Path $output 'packages'
$offlineArgument = @()
if ($Offline) {
    $offlineArgument += '--offline'
}

Push-Location $repo
try {
    cargo "+$Toolchain" package -p tweeq-core -p tweeq-slint -p tweeq-slint-build --no-verify --allow-dirty --locked @offlineArgument
    New-Item -ItemType Directory -Path $packages -Force | Out-Null
    Copy-Item -Path (Join-Path $repo 'tests/slint-consumer/*') -Destination $output -Recurse
    Copy-Item -LiteralPath (Join-Path $repo 'Cargo.lock') -Destination $output
    foreach ($name in @('tweeq-core', 'tweeq-slint', 'tweeq-slint-build')) {
        tar -xf (Join-Path $repo "target/package/$name-0.1.0-alpha.1.crate") -C $packages
    }
    $manifest = Join-Path $output 'Cargo.toml'
    # Cargo archives use reproducible timestamps. Isolate the build to ensure
    # it compiles this extraction even when the crate version is unchanged.
    $target = Join-Path $output 'target'
    cargo "+$Toolchain" check --manifest-path $manifest --target-dir $target @offlineArgument
    cargo "+$Toolchain" check --manifest-path $manifest --target-dir $target --target wasm32-unknown-unknown @offlineArgument
    if ($RunNative) {
        cargo "+$Toolchain" run --manifest-path $manifest --target-dir $target @offlineArgument -- --smoke
    }
} finally {
    Pop-Location
}
