"""Draws the app icon in a macOS Big Sur style (rounded square, soft shadow,
glossy keys) and exports assets/icon.png, assets/icon.ico and assets/icon_128.rgba.

Run from the project root:  python tools/make_icon.py
"""
from PIL import Image, ImageDraw, ImageFilter, ImageFont

S = 1024                      # canvas
BODY = (100, 100, 924, 924)   # macOS icon grid: 824px body on a 1024 canvas
RADIUS = 185
FONT = r"C:\Windows\Fonts\segoeui.ttf"
FONT_BOLD = r"C:\Windows\Fonts\segoeuib.ttf"
FONT_LIGHT = r"C:\Windows\Fonts\segoeuil.ttf"


def vgradient(size, top, bottom):
    w, h = size
    g = Image.new("RGBA", (1, h))
    for y in range(h):
        t = y / max(h - 1, 1)
        g.putpixel((0, y), tuple(int(top[i] + (bottom[i] - top[i]) * t) for i in range(4)))
    return g.resize((w, h))


def rounded_mask(size, box, radius):
    m = Image.new("L", size, 0)
    ImageDraw.Draw(m).rounded_rectangle(box, radius=radius, fill=255)
    return m


def paste_gradient(img, box, radius, top, bottom):
    x0, y0, x1, y1 = box
    grad = vgradient((x1 - x0, y1 - y0), top, bottom)
    mask = rounded_mask((x1 - x0, y1 - y0), (0, 0, x1 - x0 - 1, y1 - y0 - 1), radius)
    img.paste(grad, (x0, y0), mask)


def shadow(img, box, radius, offset, blur, alpha):
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    x0, y0, x1, y1 = box
    ImageDraw.Draw(layer).rounded_rectangle(
        (x0, y0 + offset, x1, y1 + offset), radius=radius, fill=(0, 0, 0, alpha))
    img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(blur)))


def centered_text(draw, center, label, font, fill):
    l, t, r, b = draw.textbbox((0, 0), label, font=font)
    cx, cy = center
    draw.text((cx - (l + r) / 2, cy - (t + b) / 2), label, font=font, fill=fill)


def main():
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))

    # Body: drop shadow, graphite gradient, thin top highlight.
    shadow(img, BODY, RADIUS, offset=14, blur=22, alpha=110)
    paste_gradient(img, BODY, RADIUS, (66, 70, 80, 255), (24, 25, 30, 255))
    ImageDraw.Draw(img).rounded_rectangle(
        BODY, radius=RADIUS, outline=(255, 255, 255, 40), width=3)

    # Display.
    disp = (175, 175, 849, 375)
    paste_gradient(img, disp, 48, (12, 13, 16, 255), (30, 32, 38, 255))
    d = ImageDraw.Draw(img)
    num = ImageFont.truetype(FONT_LIGHT, 150)
    l, t, r, b = d.textbbox((0, 0), "123", font=num)
    d.text((disp[2] - 50 - r, (disp[1] + disp[3]) / 2 - (t + b) / 2), "123", font=num,
           fill=(240, 242, 248, 255))

    # Keys: 3 rows x 4 columns.
    keys = [
        ["7", "8", "9", "÷"],
        ["4", "5", "6", "×"],
        ["1", "2", "3", "="],
    ]
    gx0, gy0, gx1, gy1 = 175, 420, 849, 849
    gap = 26
    kw = (gx1 - gx0 - gap * 3) / 4
    kh = (gy1 - gy0 - gap * 2) / 3
    digit_font = ImageFont.truetype(FONT, 92)
    op_font = ImageFont.truetype(FONT_BOLD, 104)
    for r_i, row in enumerate(keys):
        for c_i, label in enumerate(row):
            x0 = int(gx0 + c_i * (kw + gap))
            y0 = int(gy0 + r_i * (kh + gap))
            box = (x0, y0, int(x0 + kw), int(y0 + kh))
            operator = c_i == 3
            if label == "=":
                top, bottom = (72, 214, 140, 255), (22, 160, 92, 255)      # green
            elif operator:
                top, bottom = (86, 170, 255, 255), (20, 110, 235, 255)     # blue
            else:
                top, bottom = (112, 116, 126, 255), (78, 81, 90, 255)      # gray
            shadow(img, box, 34, offset=6, blur=6, alpha=120)
            paste_gradient(img, box, 34, top, bottom)
            d = ImageDraw.Draw(img)
            d.rounded_rectangle(box, radius=34, outline=(255, 255, 255, 45), width=2)
            centered_text(d, ((box[0] + box[2]) / 2, (box[1] + box[3]) / 2 - 4), label,
                          op_font if operator else digit_font, (255, 255, 255, 255))

    # Gloss: soft light on the upper half of the body.
    gloss = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    gmask = rounded_mask((S, S), BODY, RADIUS)
    # Fades to nothing halfway down, so there is no visible seam.
    grad = Image.new("RGBA", (S, S), (255, 255, 255, 0))
    grad.paste(vgradient((S, S // 2), (255, 255, 255, 30), (255, 255, 255, 0)), (0, 0))
    gloss.paste(grad, (0, 0), gmask)
    img.alpha_composite(gloss)

    img.resize((512, 512), Image.LANCZOS).save("assets/icon.png")
    img.save("assets/icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    with open("assets/icon_128.rgba", "wb") as f:
        f.write(img.resize((128, 128), Image.LANCZOS).tobytes())


if __name__ == "__main__":
    main()
