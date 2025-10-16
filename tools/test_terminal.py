#!/usr/bin/env python3
"""
Liquid Blobs - Smooth Version (No Flickering)
Uses cursor positioning instead of full screen clears
"""

import sys
import time
import math
import random

# ANSI codes
RESET = '\033[0m'
BOLD = '\033[1m'
HOME = '\033[H'  # Move cursor to home position
HIDE_CURSOR = '\033[?25l'
SHOW_CURSOR = '\033[?25h'
CLEAR = '\033[2J'

def rgb(r, g, b):
    """Generate RGB color ANSI code"""
    return f'\033[38;2;{int(r)};{int(g)};{int(b)}m'

def hsv_to_rgb(h, s, v):
    """Convert HSV to RGB (h: 0-360, s: 0-1, v: 0-1)"""
    c = v * s
    x = c * (1 - abs((h / 60) % 2 - 1))
    m = v - c

    if h < 60:
        r, g, b = c, x, 0
    elif h < 120:
        r, g, b = x, c, 0
    elif h < 180:
        r, g, b = 0, c, x
    elif h < 240:
        r, g, b = 0, x, c
    elif h < 300:
        r, g, b = x, 0, c
    else:
        r, g, b = c, 0, x

    return (r + m) * 255, (g + m) * 255, (b + m) * 255

class Blob:
    """A liquid blob that moves around the screen"""
    def __init__(self, x, y, radius, hue, speed_x, speed_y):
        self.x = x
        self.y = y
        self.radius = radius
        self.hue = hue
        self.speed_x = speed_x
        self.speed_y = speed_y
        self.hue_shift = random.uniform(-0.5, 0.5)

    def update(self, width, height):
        """Update blob position with bouncing"""
        self.x += self.speed_x
        self.y += self.speed_y

        # Bounce off edges with slight randomization for more organic movement
        if self.x - self.radius < 0 or self.x + self.radius > width:
            self.speed_x *= -1
            self.x = max(self.radius, min(width - self.radius, self.x))

        if self.y - self.radius < 0 or self.y + self.radius > height:
            self.speed_y *= -1
            self.y = max(self.radius, min(height - self.radius, self.y))

        # Slowly shift hue
        self.hue = (self.hue + self.hue_shift) % 360

def metaball_field(x, y, blobs):
    """Calculate metaball field strength at position (x, y)"""
    field = 0
    hue_sum = 0
    weight_sum = 0

    for blob in blobs:
        dx = x - blob.x
        dy = y - blob.y
        dist_sq = dx * dx + dy * dy

        if dist_sq < 0.01:
            dist_sq = 0.01

        # Metaball influence
        influence = (blob.radius * blob.radius) / dist_sq
        field += influence

        # Weight colors by influence
        weight = influence
        hue_sum += blob.hue * weight
        weight_sum += weight

    avg_hue = hue_sum / weight_sum if weight_sum > 0 else 0
    return field, avg_hue

def render_frame_buffer(width, height, blobs):
    """Render frame to a buffer (list of strings per line)"""
    chars = ' ·:░▒▓█'
    lines = []

    for y in range(height):
        line = ""
        for x in range(width):
            field, hue = metaball_field(x, y, blobs)

            if field < 0.3:
                line += ' '
            else:
                intensity = min(field / 3.0, 1.0)
                char_idx = int(intensity * (len(chars) - 1))
                char = chars[char_idx]

                r, g, b = hsv_to_rgb(hue, 0.8, intensity)
                line += rgb(r, g, b) + char + RESET

        lines.append(line)

    return lines

def show_title():
    """Display animated title screen"""
    print(CLEAR + HIDE_CURSOR, end='', flush=True)

    lines = [
        "",
        "╔═══════════════════════════════════════╗",
        "║                                       ║",
        "║      LIQUID BLOBS SIMULATION         ║",
        "║                                       ║",
        "║    Smooth Metaball Animation         ║",
        "║                                       ║",
        "╚═══════════════════════════════════════╝",
        "",
    ]

    for frame in range(30):
        print(HOME, end='', flush=True)
        print('\n' * 5, end='', flush=True)

        for i, line in enumerate(lines):
            hue = (frame * 12 + i * 30) % 360
            r, g, b = hsv_to_rgb(hue, 0.9, 1.0)
            print(rgb(r, g, b) + BOLD + line.center(80) + RESET, flush=True)

        time.sleep(0.033)  # ~30fps for title

    time.sleep(0.5)

def main():
    """Main animation loop"""
    width = 80
    height = 22

    # Show title
    show_title()

    # Create blobs
    blobs = [
        Blob(20, 10, 8, 0, 0.3, 0.2),
        Blob(60, 10, 6, 120, -0.2, 0.25),
        Blob(40, 15, 7, 240, 0.25, -0.3),
        Blob(30, 5, 5, 60, -0.15, 0.28),
        Blob(50, 18, 6, 300, 0.28, -0.22),
    ]

    total_frames = 300

    # Initial clear
    print(CLEAR + HIDE_CURSOR, end='', flush=True)

    for frame in range(total_frames):
        # Update blob positions
        for blob in blobs:
            blob.update(width, height)

        # Render to buffer
        frame_lines = render_frame_buffer(width, height, blobs)

        # Draw frame using cursor positioning (no clear = no flicker)
        print(HOME, end='', flush=True)

        for line in frame_lines:
            print(line, flush=True)

        # Status line
        progress = int((frame / total_frames) * 100)
        hue = (frame * 3) % 360
        r, g, b = hsv_to_rgb(hue, 0.8, 1.0)
        status = f"✨ Liquid Blobs | Frame {frame+1}/{total_frames} | {progress}%"
        print(rgb(r, g, b) + BOLD + status.center(width) + RESET, flush=True)

        time.sleep(0.033)  # Target ~30fps (ttyvid will encode at 15fps)

    # Ending
    print(CLEAR, end='', flush=True)
    print('\n' * 8, end='', flush=True)

    for i, text in enumerate(["✨ Simulation Complete! ✨", "Smooth rendering with ttyvid", ""]):
        hue = i * 60 + 180
        r, g, b = hsv_to_rgb(hue, 0.9, 1.0)
        print(rgb(r, g, b) + BOLD + text.center(80) + RESET, flush=True)
        time.sleep(0.3)

    time.sleep(1)
    print(CLEAR + SHOW_CURSOR, end='', flush=True)

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print(CLEAR + SHOW_CURSOR, end='', flush=True)
        print("\n\nAnimation interrupted.\n", flush=True)
        sys.exit(0)
