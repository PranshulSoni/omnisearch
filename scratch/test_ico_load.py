import ctypes
from ctypes import wintypes
import os
import struct

user32 = ctypes.windll.user32

# HICON CreateIconFromResourceEx(
#   PBYTE pbIconBits,
#   DWORD cbIconBits,
#   BOOL  fIcon,
#   DWORD dwVersion,
#   int   cxDesired,
#   int   cyDesired,
#   UINT  uFlags
# );
user32.CreateIconFromResourceEx.argtypes = [
    ctypes.c_void_p,
    wintypes.DWORD,
    wintypes.BOOL,
    wintypes.DWORD,
    ctypes.c_int,
    ctypes.c_int,
    wintypes.UINT
]
user32.CreateIconFromResourceEx.restype = wintypes.HICON

def test_ico_load(ico_path, size=24):
    print(f"Testing {ico_path}...")
    if not os.path.exists(ico_path):
        print("  File does not exist.")
        return
    with open(ico_path, "rb") as f:
        data = f.read()
        
    if len(data) < 6:
        print("  File too short.")
        return
        
    reserved, resource_type, count = struct.unpack("<HHH", data[:6])
    print(f"  Reserved: {reserved}, Type: {resource_type}, Count: {count}")
    
    best_idx = 0
    best_diff = 999999
    
    for i in range(count):
        offset = 6 + i * 16
        w = data[offset]
        h = data[offset+1]
        # In ICO files, width/height of 0 means 256
        if w == 0: w = 256
        if h == 0: h = 256
        diff = abs(w - size) + abs(h - size)
        if diff < best_diff:
            best_diff = diff
            best_idx = i
            
    # Get entry
    entry_offset = 6 + best_idx * 16
    w = data[entry_offset]
    h = data[entry_offset+1]
    if w == 0: w = 256
    if h == 0: h = 256
    img_size, img_offset = struct.unpack("<II", data[entry_offset+8:entry_offset+16])
    print(f"  Best match: {w}x{h} (Index {best_idx}), Size: {img_size}, Offset: {img_offset}")
    
    img_data = data[img_offset : img_offset + img_size]
    
    hicon = user32.CreateIconFromResourceEx(
        img_data,
        len(img_data),
        True,
        0x00030000,
        size,
        size,
        0
    )
    
    if hicon:
        print(f"  Success! Created HICON handle: {hicon}")
        user32.DestroyIcon(hicon)
    else:
        err = ctypes.GetLastError()
        print(f"  Failed. Error code: {err}")

def main():
    assets_dir = r"c:\Users\Pranshul Soni\Documents\Projects\Backend\Project-Raycast\assets\logo"
    settings_ico = os.path.join(assets_dir, "settings.ico")
    control_ico = os.path.join(assets_dir, "control_panel.ico")
    
    test_ico_load(settings_ico)
    test_ico_load(control_ico)

if __name__ == "__main__":
    main()
