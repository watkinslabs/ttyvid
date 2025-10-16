# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.7] - 2025-10-16

### Added
- Added `--python-unbuffered` flag to `ttyvid record` for Python script recording
  - Automatically sets `PYTHONUNBUFFERED=1` to force unbuffered output
  - Fixes issue where Python output without newlines gets buffered until program exit
  - Available in both CLI and MCP server `record` tool
- Added comprehensive Python buffering documentation in README
  - Explains line-buffered vs fully-buffered modes
  - Provides three workarounds for Python scripts
  - Includes examples of what works and what doesn't

### Documentation
- New section "Recording Python Scripts" in README explaining buffering behavior
- Added examples showing the difference between buffered and unbuffered output
- Documented three methods to work around Python buffering issues

## [0.2.6] - 2025-10-16

### Fixed
- Fixed recording not starting in current working directory - shell now spawns in the directory where `ttyvid record` was invoked

### Added
- Added `--verbose` (`-v`) flag to `ttyvid record` for detailed output
- Improved recording output formatting with minimal messages by default
  - Default: `● Recording to output.cast (Ctrl+D to stop)` and `✓ Saved to output.cast`
  - Verbose: Shows all pause/resume instructions and detailed messages

## [0.2.5] - Previous Release
