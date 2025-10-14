#!/usr/bin/env python3
"""
One-off script to make black pixels transparent in an animated GIF.
Usage: python make_transparent.py themes/layers/x.gif
"""

import sys
from PIL import Image

def make_black_transparent(input_path, output_path=None):
    """Convert black pixels to transparent in all frames of a GIF."""
    if output_path is None:
        output_path = input_path

    # Open the GIF
    img = Image.open(input_path)

    frames = []
    durations = []

    try:
        while True:
            # Get the current frame
            frame = img.convert('RGBA')

            # Get duration if available
            try:
                duration = img.info.get('duration', 100)
            except:
                duration = 100

            # Process pixels: make black transparent
            data = frame.getdata()
            new_data = []

            for item in data:
                # If pixel is black (or very dark), make it transparent
                if item[0] < 10 and item[1] < 10 and item[2] < 10:
                    new_data.append((0, 0, 0, 0))  # Transparent
                else:
                    new_data.append(item)

            frame.putdata(new_data)
            frames.append(frame)
            durations.append(duration)

            # Move to next frame
            img.seek(img.tell() + 1)

    except EOFError:
        pass  # End of sequence

    # Save all frames as animated GIF
    if frames:
        frames[0].save(
            output_path,
            save_all=True,
            append_images=frames[1:],
            duration=durations,
            loop=img.info.get('loop', 0),
            transparency=0,
            disposal=2  # Clear frame before rendering next
        )

        print(f"✓ Processed {len(frames)} frames")
        print(f"✓ Saved to: {output_path}")
    else:
        print("Error: No frames found")
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python make_transparent.py <input.gif> [output.gif]")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else input_file

    print(f"Processing: {input_file}")
    make_black_transparent(input_file, output_file)
