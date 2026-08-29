#!/usr/bin/env python3
import struct
import zlib
import math
import os

def write_png(filename, width, height, rgba_data):
    """Write an RGBA byte buffer to a valid PNG file."""
    # PNG signature
    png = bytearray(b'\x89PNG\r\n\x1a\n')
    
    # IHDR chunk: width(4), height(4), bit_depth(1), color_type(1=6:RGBA), comp(1), filter(1), interlace(1)
    ihdr_data = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)
    ihdr_crc = zlib.crc32(b'IHDR' + ihdr_data) & 0xffffffff
    png.extend(struct.pack('>I', len(ihdr_data)) + b'IHDR' + ihdr_data + struct.pack('>I', ihdr_crc))
    
    # IDAT chunk: filter byte (0) before each scanline, then scanline RGBA bytes
    raw_scanlines = bytearray()
    for y in range(height):
        raw_scanlines.append(0) # Filter type 0 (None)
        start = y * width * 4
        end = start + width * 4
        raw_scanlines.extend(rgba_data[start:end])
        
    compressed_data = zlib.compress(bytes(raw_scanlines), level=9)
    idat_crc = zlib.crc32(b'IDAT' + compressed_data) & 0xffffffff
    png.extend(struct.pack('>I', len(compressed_data)) + b'IDAT' + compressed_data + struct.pack('>I', idat_crc))
    
    # IEND chunk
    iend_crc = zlib.crc32(b'IEND') & 0xffffffff
    png.extend(struct.pack('>I', 0) + b'IEND' + struct.pack('>I', iend_crc))
    
    with open(filename, 'wb') as f:
        f.write(png)

def render_icon(size, maskable=False):
    """Render high-fidelity bList icon with smooth antialiasing and gradient shading."""
    width, height = size, size
    buf = bytearray(width * height * 4)
    
    # Scale factor
    s = size / 512.0
    
    # Radii and positions in 512 space
    corner_r = 0 if maskable else 115 * s
    scale_inner = 0.82 if maskable else 1.0
    offset_inner = (512 * (1.0 - scale_inner) / 2.0) * s
    
    for y in range(height):
        ny = y / height
        for x in range(width):
            nx = x / width
            idx = (y * width + x) * 4
            
            # Squircle / rounded rect calculation for non-maskable
            inside_bg = True
            alpha_bg = 1.0
            
            if not maskable:
                # Rounded rect distance
                dx = max(0, abs(x - width/2) - (width/2 - corner_r))
                dy = max(0, abs(y - height/2) - (height/2 - corner_r))
                dist = math.sqrt(dx*dx + dy*dy)
                if dist > corner_r:
                    # Antialiasing edge
                    edge = dist - corner_r
                    if edge < 1.0:
                        alpha_bg = max(0.0, 1.0 - edge)
                    else:
                        inside_bg = False
                        alpha_bg = 0.0
                        
            if not inside_bg:
                buf[idx] = 0
                buf[idx+1] = 0
                buf[idx+2] = 0
                buf[idx+3] = 0
                continue
                
            # Background Gradient from #3b82f6 (59, 130, 246) -> #1d4ed8 (29, 78, 216)
            diag = (nx + ny) / 2.0
            r_bg = int(59 * (1.0 - diag) + 29 * diag)
            g_bg = int(130 * (1.0 - diag) + 78 * diag)
            b_bg = int(246 * (1.0 - diag) + 216 * diag)
            
            # Subtle center radial glow
            cx_glow = width * 0.5
            cy_glow = height * 0.38
            dist_glow = math.sqrt((x - cx_glow)**2 + (y - cy_glow)**2) / (width * 0.65)
            if dist_glow < 1.0:
                glow_int = (1.0 - dist_glow) * 0.35
                r_bg = min(255, int(r_bg + 60 * glow_int))
                g_bg = min(255, int(g_bg + 80 * glow_int))
                b_bg = min(255, int(b_bg + 40 * glow_int))
                
            # Current pixel color
            r, g, b, a = r_bg, g_bg, b_bg, int(255 * alpha_bg)
            
            # Normalized coordinates inside pin space
            # Translate & scale to inner 512 space
            px = ((x - offset_inner) / (s * scale_inner))
            py = ((y - offset_inner) / (s * scale_inner))
            
            # Map Pin Geometry in 512 Space:
            # Pin center head at (256, 215), radius 146
            # Tip at (256, 426)
            # Tangent lines from tip to circle
            dx_pin = px - 256.0
            dy_pin = py - 215.0
            dist_pin_head = math.sqrt(dx_pin*dx_pin + dy_pin*dy_pin)
            
            inside_pin = False
            pin_alpha = 0.0
            
            if dist_pin_head <= 146.0:
                inside_pin = True
                pin_alpha = 1.0
                if 146.0 - dist_pin_head < 1.5:
                    pin_alpha = (146.0 - dist_pin_head) / 1.5
            elif py >= 215.0 and py <= 428.0:
                # Triangular taper to tip
                progress = (py - 215.0) / (428.0 - 215.0)
                allowed_w = 146.0 * (1.0 - math.pow(progress, 0.75))
                if abs(dx_pin) <= allowed_w:
                    inside_pin = True
                    pin_alpha = 1.0
                    if allowed_w - abs(dx_pin) < 1.5:
                        pin_alpha = (allowed_w - abs(dx_pin)) / 1.5
                        
            if inside_pin and pin_alpha > 0.0:
                # White/soft gradient for pin body
                pin_grad = min(1.0, max(0.0, (py - 70.0) / 350.0))
                pr = int(255 * (1.0 - pin_grad) + 230 * pin_grad)
                pg = int(255 * (1.0 - pin_grad) + 235 * pin_grad)
                pb = 255
                
                # Inner Core Circle (radius 75 at 256, 215)
                dist_core = math.sqrt(dx_pin*dx_pin + dy_pin*dy_pin)
                if dist_core <= 75.0:
                    core_alpha = 1.0
                    if 75.0 - dist_core < 1.5:
                        core_alpha = (75.0 - dist_core) / 1.5
                    
                    # Inner core deep blue #2563eb / #1e40af
                    cr = 37
                    cg = 99
                    cb = 235
                    
                    # Checkmark inside core: line from (230, 215) -> (248, 233) -> (285, 194)
                    # Test distance to line segments
                    def dist_to_segment(x, y, x1, y1, x2, y2):
                        l2 = (x2-x1)**2 + (y2-y1)**2
                        if l2 == 0: return math.sqrt((x-x1)**2 + (y-y1)**2)
                        t = max(0, min(1, ((x-x1)*(x2-x1) + (y-y1)*(y2-y1)) / l2))
                        proj_x = x1 + t*(x2-x1)
                        proj_y = y1 + t*(y2-y1)
                        return math.sqrt((x-proj_x)**2 + (y-proj_y)**2)
                        
                    d_chk1 = dist_to_segment(px, py, 230, 215, 248, 233)
                    d_chk2 = dist_to_segment(px, py, 248, 233, 285, 194)
                    d_chk = min(d_chk1, d_chk2)
                    
                    if d_chk <= 7.0:
                        chk_a = 1.0
                        if 7.0 - d_chk < 1.5:
                            chk_a = (7.0 - d_chk) / 1.5
                        cr = int(cr * (1.0 - chk_a) + 255 * chk_a)
                        cg = int(cg * (1.0 - chk_a) + 255 * chk_a)
                        cb = int(cb * (1.0 - chk_a) + 255 * chk_a)
                        
                    pr = int(pr * (1.0 - core_alpha) + cr * core_alpha)
                    pg = int(pg * (1.0 - core_alpha) + cg * core_alpha)
                    pb = int(pb * (1.0 - core_alpha) + cb * core_alpha)
                    
                # Blend pin over background
                r = int(r * (1.0 - pin_alpha) + pr * pin_alpha)
                g = int(g * (1.0 - pin_alpha) + pg * pin_alpha)
                b = int(b * (1.0 - pin_alpha) + pb * pin_alpha)

            # Star Badge at top right (335, 120)
            star_dx = px - 335.0
            star_dy = py - 120.0
            star_dist = math.sqrt(star_dx*star_dx + star_dy*star_dy)
            if star_dist <= 26.0:
                s_alpha = 1.0
                if 26.0 - star_dist < 1.5:
                    s_alpha = (26.0 - star_dist) / 1.5
                sr, sg, sb = 255, 255, 255
                if star_dist <= 21.0:
                    sr, sg, sb = 245, 158, 11 # Amber #f59e0b
                r = int(r * (1.0 - s_alpha) + sr * s_alpha)
                g = int(g * (1.0 - s_alpha) + sg * s_alpha)
                b = int(b * (1.0 - s_alpha) + sb * s_alpha)

            buf[idx] = r
            buf[idx+1] = g
            buf[idx+2] = b
            buf[idx+3] = a
            
    return bytes(buf)

def main():
    icons_dir = "/home/spezd/projects/map-bucket-list/static/icons"
    os.makedirs(icons_dir, exist_ok=True)
    
    print("🎨 Generating bList high-resolution icon set...")
    
    # 1. 192x192 Standard Icon
    print("  -> Generating icon-192.png")
    data_192 = render_icon(192, maskable=False)
    write_png(os.path.join(icons_dir, "icon-192.png"), 192, 192, data_192)
    
    # 2. 512x512 Standard Icon
    print("  -> Generating icon-512.png")
    data_512 = render_icon(512, maskable=False)
    write_png(os.path.join(icons_dir, "icon-512.png"), 512, 512, data_512)
    
    # 3. 192x192 Maskable Icon
    print("  -> Generating icon-maskable-192.png")
    data_mask_192 = render_icon(192, maskable=True)
    write_png(os.path.join(icons_dir, "icon-maskable-192.png"), 192, 192, data_mask_192)
    
    # 4. 512x512 Maskable Icon
    print("  -> Generating icon-maskable-512.png")
    data_mask_512 = render_icon(512, maskable=True)
    write_png(os.path.join(icons_dir, "icon-maskable-512.png"), 512, 512, data_mask_512)
    
    # 5. Apple Touch Icon (180x180)
    print("  -> Generating apple-touch-icon.png (180x180)")
    data_180 = render_icon(180, maskable=False)
    write_png(os.path.join(icons_dir, "apple-touch-icon.png"), 180, 180, data_180)
    
    # 6. Favicon (64x64) & Favicon (32x32)
    print("  -> Generating favicon.png (64x64)")
    data_64 = render_icon(64, maskable=False)
    write_png(os.path.join(icons_dir, "favicon.png"), 64, 64, data_64)
    write_png(os.path.join(icons_dir, "favicon-32.png"), 64, 64, data_64)

    print("✨ Icon generation complete!")

if __name__ == "__main__":
    main()
