import math
import os
import numpy as np
import mathutils.noise

def generate_aphora_texture():
    width = 2048
    height = 1024
    print(f"Generating Aphora planet texture ({width}x{height})...")

    img_data = np.zeros((height, width, 4), dtype=np.uint8)

    # Palette defining tones of pink and purple
    # Colors: [Deep Plum, Dark Violet, Magenta, Hot Pink, Soft Pink, Electric Purple, Lavender]
    c0 = np.array([35, 10, 50])     # Deep Plum / Night Purple
    c1 = np.array([75, 15, 95])     # Dark Violet
    c2 = np.array([140, 25, 130])   # Rich Purple
    c3 = np.array([215, 40, 160])   # Magenta
    c4 = np.array([245, 100, 190])  # Hot Pink
    c5 = np.array([250, 180, 225])  # Soft Lavender/Pink
    c6 = np.array([255, 230, 250])  # Bright Pink-White Highlight

    # Generate 3D sphere coordinate grid
    u = np.linspace(0, 1, width, endpoint=False)
    v = np.linspace(0, 1, height, endpoint=False)
    grid_u, grid_v = np.meshgrid(u, v)

    theta = grid_v * math.pi      # 0 to pi (lat)
    phi = grid_u * 2.0 * math.pi  # 0 to 2pi (lon)

    sin_theta = np.sin(theta)
    x = sin_theta * np.cos(phi)
    y = sin_theta * np.sin(phi)
    z = np.cos(theta)

    for i in range(height):
        v_val = v[i]
        lat_angle = (v_val - 0.5) * math.pi
        band_base = math.sin(lat_angle * 14.0) * 0.5 + 0.5

        for j in range(width):
            px = x[i, j]
            py = y[i, j]
            pz = z[i, j]

            # Multi-octave Perlin noise in 3D
            p_vec = mathutils.Vector((px * 3.5, py * 3.5, pz * 3.5))
            n1 = mathutils.noise.fractal(p_vec, 2.0, 2.0, 6) # Main fractal
            
            p_swirl = mathutils.Vector((px * 8.0 + n1 * 1.5, py * 8.0, pz * 8.0 + n1 * 1.5))
            n2 = mathutils.noise.turbulence(p_swirl, 5, True) # Fine turbulence

            # Combine band base + noise
            val = (band_base * 0.4) + (n1 * 0.4) + (n2 * 0.2)
            val = max(0.0, min(1.0, val))

            # Color interpolation through 6 color stops
            if val < 0.2:
                t = val / 0.2
                col = c0 * (1 - t) + c1 * t
            elif val < 0.4:
                t = (val - 0.2) / 0.2
                col = c1 * (1 - t) + c2 * t
            elif val < 0.6:
                t = (val - 0.4) / 0.2
                col = c2 * (1 - t) + c3 * t
            elif val < 0.8:
                t = (val - 0.6) / 0.2
                col = c3 * (1 - t) + c4 * t
            elif val < 0.95:
                t = (val - 0.8) / 0.15
                col = c4 * (1 - t) + c5 * t
            else:
                t = (val - 0.95) / 0.05
                col = c5 * (1 - t) + c6 * t

            img_data[i, j, 0] = int(min(255, max(0, col[0])))
            img_data[i, j, 1] = int(min(255, max(0, col[1])))
            img_data[i, j, 2] = int(min(255, max(0, col[2])))
            img_data[i, j, 3] = 255

        if i % 128 == 0:
            print(f"  Row {i}/{height} done...")

    # Save image using Blender API or simple PNG writer
    import bpy
    blender_img = bpy.data.images.new("aphora_tex", width=width, height=height, alpha=True)
    pixels = (img_data.astype(np.float32) / 255.0).flatten()
    blender_img.pixels = pixels.tolist()

    out_path = os.path.abspath("assets/textures/aphora.png")
    blender_img.filepath_raw = out_path
    blender_img.file_format = 'PNG'
    blender_img.save()
    print(f"Saved Aphora texture to {out_path}")

def generate_aphora_ring_textures():
    width = 1024
    height = 64
    print("Generating Aphora ring textures...")

    # Ring 1: Inner ring (Hot Pink / Magenta with bright bands)
    img_data1 = np.zeros((height, width, 4), dtype=np.uint8)
    # Ring 2: Outer ring (Deep Purple / Violet with subtle stripes)
    img_data2 = np.zeros((height, width, 4), dtype=np.uint8)

    for j in range(width):
        r = j / float(width) # 0 inner edge to 1 outer edge
        
        # Radial transparency and banding for Ring 1
        alpha1 = math.sin(r * math.pi) * 0.85
        n1 = math.sin(r * 45.0) * 0.15 + math.sin(r * 110.0) * 0.08
        intensity1 = max(0.0, min(1.0, 0.7 + n1))
        
        r1_col = np.array([235, 60, 180]) * intensity1
        a1_val = int(max(0, min(255, alpha1 * 255)))

        for i in range(height):
            img_data1[i, j, 0] = int(r1_col[0])
            img_data1[i, j, 1] = int(r1_col[1])
            img_data1[i, j, 2] = int(r1_col[2])
            img_data1[i, j, 3] = a1_val

        # Radial transparency and banding for Ring 2
        alpha2 = math.sin(r * math.pi) * 0.75
        n2 = math.cos(r * 60.0) * 0.18 + math.sin(r * 140.0) * 0.1
        intensity2 = max(0.0, min(1.0, 0.65 + n2))
        
        r2_col = np.array([160, 50, 220]) * intensity2
        a2_val = int(max(0, min(255, alpha2 * 255)))

        for i in range(height):
            img_data2[i, j, 0] = int(r2_col[0])
            img_data2[i, j, 1] = int(r2_col[1])
            img_data2[i, j, 2] = int(r2_col[2])
            img_data2[i, j, 3] = a2_val

    import bpy
    # Save Ring 1
    ring1_img = bpy.data.images.new("aphora_ring1", width=width, height=height, alpha=True)
    ring1_img.pixels = (img_data1.astype(np.float32) / 255.0).flatten().tolist()
    path1 = os.path.abspath("assets/textures/aphora_ring1.png")
    ring1_img.filepath_raw = path1
    ring1_img.file_format = 'PNG'
    ring1_img.save()

    # Save Ring 2
    ring2_img = bpy.data.images.new("aphora_ring2", width=width, height=height, alpha=True)
    ring2_img.pixels = (img_data2.astype(np.float32) / 255.0).flatten().tolist()
    path2 = os.path.abspath("assets/textures/aphora_ring2.png")
    ring2_img.filepath_raw = path2
    ring2_img.file_format = 'PNG'
    ring2_img.save()
    print("Saved ring textures.")

if __name__ == "__main__":
    generate_aphora_texture()
    generate_aphora_ring_textures()
