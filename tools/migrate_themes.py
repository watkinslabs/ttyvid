#!/usr/bin/env python3
"""
Migrate Python .theme files to YAML format.
"""

import sys
import yaml
from pathlib import Path
from typing import Dict, Any, List, Optional


class ThemeParser:
    def __init__(self, theme_path: Path):
        self.theme_path = theme_path
        self.lines = theme_path.read_text().splitlines()
        self.index = 0

    def parse(self) -> Dict[str, Any]:
        theme = {}
        current_section = None

        while self.index < len(self.lines):
            line = self.lines[self.index].strip()
            self.index += 1

            # Skip empty lines and comments
            if not line or line.startswith('#'):
                continue

            # Check for section headers
            if line == 'title':
                theme['title'] = self._parse_title()
            elif line == 'padding':
                theme['padding'] = self._parse_padding()
            elif line == 'layer':
                if 'layers' not in theme:
                    theme['layers'] = []
                theme['layers'].append(self._parse_layer())
            elif line == 'palette':
                theme['palette'] = self._parse_palette()
            else:
                # Top-level key-value pair
                parts = line.split(None, 1)
                if len(parts) == 2:
                    key, value = parts
                    theme[key] = self._convert_value(value)

        return theme

    def _parse_title(self) -> Dict[str, Any]:
        title = {}
        while self.index < len(self.lines):
            line = self.lines[self.index].strip()

            if not line or line.startswith('#'):
                self.index += 1
                continue

            # Check if we've reached a new section
            if line in ['padding', 'layer', 'palette']:
                break

            parts = line.split(None, 1)
            if len(parts) == 2:
                key, value = parts
                title[key] = self._convert_value(value)

            self.index += 1

        return title

    def _parse_padding(self) -> Dict[str, Any]:
        padding = {}
        while self.index < len(self.lines):
            line = self.lines[self.index].strip()

            if not line or line.startswith('#'):
                self.index += 1
                continue

            # Check if we've reached a new section
            if line in ['title', 'layer', 'palette']:
                break

            parts = line.split(None, 1)
            if len(parts) == 2:
                key, value = parts
                padding[key] = self._convert_value(value)

            self.index += 1

        return padding

    def _parse_layer(self) -> Dict[str, Any]:
        layer = {}
        nineslice = {}
        bounds = {}
        dst_bounds = {}

        while self.index < len(self.lines):
            line = self.lines[self.index].strip()

            if not line or line.startswith('#'):
                self.index += 1
                continue

            # Check if we've reached a new section
            if line in ['title', 'padding', 'layer', 'palette']:
                break

            parts = line.split(None, 1)
            if len(parts) == 2:
                key, value = parts

                # Handle 9-slice outer/inner coords
                if key.startswith('outer-') or key.startswith('inner-'):
                    nineslice_key = key.replace('-', '_')
                    nineslice[nineslice_key] = self._convert_value(value)
                # Handle bounds
                elif key in ['left', 'top', 'right', 'bottom']:
                    bounds[key] = self._convert_value(value)
                # Handle dst-bounds
                elif key.startswith('dst-'):
                    dst_key = key.replace('dst-', '')
                    dst_bounds[dst_key] = self._convert_value(value)
                # Handle copy-mode
                elif key == 'copy-mode':
                    layer['copy_mode'] = value
                # Handle file path - add layers/ prefix if not present
                elif key == 'file':
                    file_value = value.strip()
                    if not file_value.startswith('layers/'):
                        file_value = f'layers/{file_value}'
                    layer['file'] = file_value
                else:
                    layer[key] = self._convert_value(value)

            self.index += 1

        # Add nineslice if it has data
        if nineslice:
            layer['nineslice'] = nineslice

        # Add bounds if it has data
        if bounds:
            layer['bounds'] = bounds

        # Add dst_bounds if it has data
        if dst_bounds:
            layer['dst_bounds'] = dst_bounds

        return layer

    def _parse_palette(self) -> Dict[str, Any]:
        palette = {'rgb': []}

        while self.index < len(self.lines):
            line = self.lines[self.index].strip()

            if not line or line.startswith('#'):
                self.index += 1
                continue

            # Check if we've reached a new section
            if line in ['title', 'padding', 'layer']:
                break

            # Check for "colors N" line
            parts = line.split()
            if len(parts) == 2 and parts[0] == 'colors':
                palette['colors'] = int(parts[1])
                self.index += 1
                continue

            # Parse RGB triplet
            if len(parts) == 3:
                try:
                    r, g, b = int(parts[0]), int(parts[1]), int(parts[2])
                    palette['rgb'].append([r, g, b])
                except ValueError:
                    pass

            self.index += 1

        return palette

    def _convert_value(self, value: str) -> Any:
        """Convert string values to appropriate types."""
        value = value.strip()

        # Handle 'auto'
        if value.lower() == 'auto':
            return 'auto'

        # Try to convert to int
        try:
            return int(value)
        except ValueError:
            pass

        # Try to convert to float
        try:
            return float(value)
        except ValueError:
            pass

        # Return as string
        return value


def migrate_theme(theme_path: Path, output_path: Path):
    """Migrate a single theme file."""
    parser = ThemeParser(theme_path)
    theme_data = parser.parse()

    # Write as YAML
    with output_path.open('w') as f:
        f.write(f"# {theme_data.get('name', 'Theme')}\n")
        f.write(f"# Migrated from: {theme_path.name}\n\n")
        yaml.dump(theme_data, f, default_flow_style=False, sort_keys=False, allow_unicode=True)

    print(f"✓ Migrated: {theme_path.name} -> {output_path.name}")


def main():
    # Paths
    python_themes_dir = Path("../../ttygif/ttygif/tty/themes")
    rust_themes_dir = Path("themes")

    if not python_themes_dir.exists():
        print(f"Error: Python themes directory not found: {python_themes_dir}")
        sys.exit(1)

    rust_themes_dir.mkdir(exist_ok=True)

    # Find all .theme files
    theme_files = list(python_themes_dir.glob("*.theme"))

    if not theme_files:
        print(f"No .theme files found in {python_themes_dir}")
        sys.exit(1)

    print(f"Found {len(theme_files)} theme files to migrate\n")

    # Migrate each theme
    for theme_file in sorted(theme_files):
        output_file = rust_themes_dir / f"{theme_file.stem}.yaml"
        try:
            migrate_theme(theme_file, output_file)
        except Exception as e:
            print(f"✗ Failed to migrate {theme_file.name}: {e}")

    print(f"\n✓ Migration complete! Themes saved to {rust_themes_dir}/")


if __name__ == '__main__':
    main()
