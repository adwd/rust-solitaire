"""Regenerate table and highlight textures with Pillow.

The classic deck is rendered separately by render_cards.cjs from the bundled
CC0 SVGs. All finished artwork is embedded, with no runtime file dependencies.
"""
from pathlib import Path
from PIL import Image, ImageDraw
import math

OUT = Path(__file__).resolve().parents[1] / "assets"
OUT.mkdir(exist_ok=True)
W, H = 320, 448


def suit(d, kind, x, y, size, fill):
    r = size / 2
    if kind == 1:
        d.polygon([(x, y-r), (x+r*.7, y), (x, y+r), (x-r*.7, y)], fill=fill)
    elif kind in (2, 3):
        pts = []
        for i in range(81):
            t = i * math.tau / 80
            hx = 16 * math.sin(t)**3 / 18
            hy = -(13*math.cos(t)-5*math.cos(2*t)-2*math.cos(3*t)-math.cos(4*t)) / 18
            if kind == 3:
                hy = -hy
            pts.append((x+hx*r, y+hy*r*.95))
        d.polygon(pts, fill=fill)
        if kind == 3:
            d.polygon([(x-r*.19,y+r*.28),(x-r*.32,y+r*.93),(x+r*.32,y+r*.93),(x+r*.19,y+r*.28)], fill=fill)
    else:
        for dx,dy in [(0,-.38),(-.36,.12),(.36,.12)]:
            d.ellipse((x+(dx-.4)*r,y+(dy-.4)*r,x+(dx+.4)*r,y+(dy+.4)*r),fill=fill)
        d.polygon([(x-r*.14,y),(x-r*.3,y+r*.95),(x+r*.3,y+r*.95),(x+r*.14,y)],fill=fill)


highlight = Image.new("RGBA", (W,H), (0,0,0,0))
ImageDraw.Draw(highlight).rounded_rectangle((3,3,W-4,H-4),radius=20,outline=(225,196,126,255),width=8)
highlight.save(OUT / "highlight.png",optimize=True)

for index in range(6):
    im = Image.new("RGBA",(W,H),(0,0,0,0))
    d = ImageDraw.Draw(im)
    d.rounded_rectangle((3,3,W-4,H-4),radius=20,fill=(20,72,60,255),outline=(70,113,94,255),width=3)
    if index < 4:
        suit(d,index,160,224,100,(70,113,94,255))
    elif index == 4:
        d.arc((105,171,215,281),25,325,fill=(70,113,94,255),width=5)
        d.polygon([(205,169),(217,202),(187,190)],fill=(70,113,94,255))
    im.save(OUT / f"slot{index}.png",optimize=True)

# Match klondike-egui's FELT exactly. Lighting is neutral and non-specular.
felt = Image.new("RGB", (64,64), (16,65,55))
felt.save(OUT / "felt.png", optimize=True)
print(f"Generated slots, highlight and web-palette felt in {OUT}")
