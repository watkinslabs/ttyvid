# Extended Font Collection

This directory contains converted bitmap fonts in the `.fd` format for use with ttyvid.

## Available Fonts

All fonts are converted at 16px size with full anti-aliasing support (0-10 intensity scale).

### Adwaita Mono (10x21)
- **File**: `Adwaita_Mono_16.fd`
- **License**: SIL Open Font License 1.1 (OFL)
- **License File**: `licenses/Adwaita-Mono-LICENSE.md`
- **Copyright**: © 2015-2025 Renzhi Li (aka. Belleve Invis)
- **Description**: Clean, modern monospace font designed for the GNOME desktop

### Liberation Mono (11x19)
- **File**: `Liberation_Mono_16.fd`
- **License**: SIL Open Font License 1.1 (OFL)
- **License File**: `licenses/Liberation-Mono-LICENSE.txt`
- **Copyright**: © 2010 Google Corporation, © 2012 Red Hat, Inc.
- **Description**: Metrically compatible with Courier New, excellent for compatibility

### Noto Sans Mono (11x20)
- **File**: `Noto_Sans_Mono_16.fd`
- **License**: Apache License 2.0
- **License File**: `licenses/Noto-Sans-Mono-LICENSE.txt`
- **Copyright**: © Google
- **Description**: Part of the Noto font family, designed for broad language support

### Source Code Pro (12x25)
- **File**: `Source_Code_Pro_16.fd`
- **License**: SIL Open Font License 1.1 (OFL)
- **License File**: `licenses/Source-Code-Pro-LICENSE.md`
- **Copyright**: © 2023 Adobe
- **Description**: Professionally designed monospace font optimized for coding

## Usage

To use these fonts with ttyvid, specify the font file path:

```bash
ttyvid -i input.cast -o output.gif --font-file themes/font_Ext/Source_Code_Pro_16.fd
```

## License Information

All fonts in this collection are freely licensed and may be redistributed:
- **SIL Open Font License 1.1**: Allows free use, modification, and redistribution
- **Apache License 2.0**: Allows free use, modification, and redistribution

See the individual license files in the `licenses/` directory for complete terms.

## Font Conversion

These fonts were converted from TrueType/OpenType format to the `.fd` bitmap format using:

```bash
ttyvid convert-font --font <font-path> --output <output.fd> --size 16
```

The conversion process:
- Renders each character at 16px height
- Captures anti-aliasing with 11 intensity levels (0-10 scale)
- Supports full Unicode character set
- Generates optimized bitmap data with tight bounding boxes
