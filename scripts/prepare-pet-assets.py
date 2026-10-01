"""Prepare downloaded character art without drawing or generating character pixels.

Run explicitly with Python + Pillow. App/builds use the committed local WebP files;
they never contact the providers. Cached downloads are kept outside public/.
"""
from collections import deque
from hashlib import sha256
from io import BytesIO
import json
from pathlib import Path
import urllib.request

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "public" / "pets"
CACHE = ROOT / "artifacts" / "pet-source-cache"


def sticker(name, ident, publisher, owner, **options):
    return dict(name=name, url=f"https://media.giphy.com/media/{ident}/giphy.gif",
                page=f"https://giphy.com/stickers/{ident}", publisher=publisher,
                owner=owner, **options)


SOURCES = [
    dict(name="remy", url="https://www.pngkit.com/png/detail/938-9388574_ratatouille-ratatouille-remy.png",
         page="https://www.pngkit.com/view/u2y3a9t4y3w7r5q8_ratatouille-ratatouille-remy/",
         publisher="PNGkit (community-hosted character render)", owner="Disney / Pixar", matte="white"),
    dict(name="emile", url="https://w7.pngwing.com/pngs/319/12/png-transparent-ratatouille-emile-remy-pixar-animated-film-ratatuille-mammal-food-mouse.png",
         page="https://www.pngwing.com/en/free-png-bmmxp", publisher="PNGWing (community-hosted character render)", owner="Disney / Pixar", matte="white"),
    sticker("pikachu", "FCffpN404oRZpFbSzl", "Pokémon", "The Pokémon Company"),
    sticker("eevee", "Nk39jnb19T7MjcWTBi", "Pokémon", "The Pokémon Company"),
    sticker("bulbasaur", "eN6H6Eszm15OwKEONx", "Pokémon", "The Pokémon Company"),
    sticker("charmander", "6xr4bW2csldWKmJkjO", "Pokémon", "The Pokémon Company"),
    sticker("squirtle", "sIo7BCXxPDPNDfS3dE", "Pokémon", "The Pokémon Company"),
    sticker("jigglypuff", "70156xKexe2CqBOOSB", "Pokémon", "The Pokémon Company"),
    sticker("snoopy", "daOM5veK5Mwasb5TLY", "Peanuts", "Peanuts Worldwide LLC", crop=[0, 0, 245, 270]),
    sticker("snoopy-dance", "4bz6frJ6gNlCUEExIu", "Peanuts", "Peanuts Worldwide LLC"),
    sticker("woodstock", "jptAHfCnH8rSgVSjcE", "Peanuts", "Peanuts Worldwide LLC"),
    sticker("belle", "3UPF60aOXPYbpljc9m", "Peanuts", "Peanuts Worldwide LLC", crop=[241, 65, 449, 357], matte="yellow", component=True),
    dict(name="remy-cheese", url="https://media.giphy.com/media/PylzSrsqgmmqkisroH/giphy.gif",
         page="https://giphy.com/gifs/PylzSrsqgmmqkisroH", publisher="Disney", owner="Disney / Pixar", frames=[0, 14], film=True),
    dict(name="remy-strawberry", url="https://media.giphy.com/media/PylzSrsqgmmqkisroH/giphy.gif",
         page="https://giphy.com/gifs/PylzSrsqgmmqkisroH", publisher="Disney", owner="Disney / Pixar", frames=[14, 28], film=True),
    dict(name="emile-snack", url="https://media.giphy.com/media/jnkvKeUHWnRYs/giphy.gif",
         page="https://giphy.com/gifs/jnkvKeUHWnRYs", publisher="Disney Pixar", owner="Disney / Pixar", crop=[250, 0, 500, 187], film=True),
]

# Quiet variants use a short, neutral sequence of original frames, gently return
# along the same frames, then dwell. No character poses are redrawn.
for character in ["pikachu", "eevee", "bulbasaur", "charmander", "squirtle", "jigglypuff", "snoopy", "woodstock", "belle"]:
    base = next(spec for spec in SOURCES if spec["name"] == character)
    SOURCES.append({**base, "name": character + "-quiet", "frames": [0, 7 if character == "pikachu" else 6],
                    "timeScale": 4, "pingPong": True, "holdMs": 4000, "wholeSourceBounds": True})
SOURCES.append({**next(spec for spec in SOURCES if spec["name"] == "pikachu"),
                "name": "pikachu-idle", "frames": [0, 7], "timeScale": 2, "pingPong": True, "holdMs": 2200, "wholeSourceBounds": True})


def remove_matte(frame, color, component=False):
    """Remove an existing flat background; keep the source character pixels."""
    pixels = list(frame.get_flattened_data())
    if color == "yellow":
        mask = Image.new("L", frame.size)
        mask.putdata([255 if a and not (r > 120 and g > 85 and b < 150 and r > b * 1.15 and g > b * 1.15) else 0
                      for r, g, b, a in pixels])
        if component:
            # Keep Belle's connected silhouette, excluding lettering/other dogs.
            w, h = mask.size
            bits = bytearray(mask.tobytes())
            largest = []
            for start in range(len(bits)):
                if bits[start] != 255:
                    continue
                queue = deque([start]); bits[start] = 0; found = []
                while queue:
                    i = queue.popleft(); found.append(i)
                    x, y = i % w, i // w
                    for n in ([i - 1] if x else []) + ([i + 1] if x + 1 < w else []) + ([i - w] if y else []) + ([i + w] if y + 1 < h else []):
                        if bits[n] == 255:
                            bits[n] = 0; queue.append(n)
                if len(found) > len(largest):
                    largest = found
            kept = bytearray(w * h)
            for i in largest:
                kept[i] = 255
            mask = Image.frombytes("L", (w, h), bytes(kept))
    else:
        # Flood only the exterior light background; leave eyes/highlights intact.
        mask = Image.new("L", frame.size)
        mask.putdata([255 if min(r, g, b) >= 226 and max(r, g, b) - min(r, g, b) < 25 else 0
                      for r, g, b, _ in pixels])
        ImageDraw.floodfill(mask, (0, 0), 128)
        mask.putdata([0 if p == 128 else 255 for p in mask.get_flattened_data()])
    frame.putalpha(mask)
    return frame


def source_frame(image, spec):
    frame = image.convert("RGBA")
    if "crop" in spec:
        frame = frame.crop(spec["crop"])
    if "matte" in spec:
        frame = remove_matte(frame, spec["matte"], spec.get("component", False))
    return frame


def prepare(spec):
    cached = CACHE / (sha256(spec["url"].encode()).hexdigest()[:20] + ".source")
    if not cached.exists():
        request = urllib.request.Request(spec["url"], headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(request, timeout=30) as response:
            cached.write_bytes(response.read())
    raw = cached.read_bytes()
    image = Image.open(BytesIO(raw))
    source_size = list(image.size)
    frames, durations = [], []
    first, last = spec.get("frames", [0, getattr(image, "n_frames", 1)])
    for index in range(first, last):
        image.seek(index)
        frames.append(source_frame(image, spec))
        durations.append(max(40, int(image.info.get("duration", 100))) * spec.get("timeScale", 1))
    if spec.get("pingPong") and len(frames) > 2:
        frames += [frame.copy() for frame in frames[-2:0:-1]]
        durations += durations[-2:0:-1]
    durations[0] += spec.get("holdMs", 0)
    # One shared crop across frames prevents a wobbling scale/foot position.
    boxes = [frame.getbbox() for frame in frames if frame.getbbox()]
    if spec.get("wholeSourceBounds"):
        boxes = []
        for index in range(getattr(image, "n_frames", 1)):
            image.seek(index)
            bbox = source_frame(image, spec).getbbox()
            if bbox:
                boxes.append(bbox)
    assert boxes, f"No visible pixels in {spec['name']}"
    box = (max(0, min(b[0] for b in boxes) - 6), max(0, min(b[1] for b in boxes) - 6),
           min(frames[0].width, max(b[2] for b in boxes) + 6), min(frames[0].height, max(b[3] for b in boxes) + 6))
    prepared = []
    for frame in frames:
        frame = frame.crop(box)
        frame.thumbnail((480, 480), Image.Resampling.LANCZOS)
        prepared.append(frame)
    name = spec["name"]
    poster = OUT / (name + "-poster.webp")
    prepared[0].save(poster, format="WEBP", lossless=True, method=6)
    animation = None
    if len(prepared) > 1:
        animation = OUT / (name + ".webp")
        prepared[0].save(animation, format="WEBP", save_all=True, append_images=prepared[1:],
                         duration=durations, loop=0, lossless=True, method=6)
    return {**spec, "sourceSha256": sha256(raw).hexdigest(), "sourceSize": source_size,
            "preparedSize": list(prepared[0].size), "pixelCrop": list(box), "frameCount": len(prepared),
            "durationMs": sum(durations), "poster": "/pets/" + poster.name,
            "animation": "/pets/" + animation.name if animation else None,
            "files": {p.name: {"sha256": sha256(p.read_bytes()).hexdigest(), "bytes": p.stat().st_size}
                      for p in [poster, animation] if p}}


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    CACHE.mkdir(parents=True, exist_ok=True)
    assets = []
    for spec in SOURCES:
        item = prepare(spec); assets.append(item)
        print(spec["name"], item["preparedSize"], item["frameCount"], flush=True)
    (OUT / "sources.json").write_text(json.dumps({"assets": assets}, indent=2) + "\n", encoding="utf-8")
    total = sum(file["bytes"] for item in assets for file in item["files"].values())
    print(f"Local character assets: {total / 1024 / 1024:.2f} MiB")
