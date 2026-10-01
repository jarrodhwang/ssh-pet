# Character art and animation sources

Character pixels were sourced from existing internet artwork, not drawn or generated for this app. Droplet remains the app's original SVG. Copyright stays with the respective owners. Source hosting is attribution, not a transfer of copyright or a redistribution license; PNGWing lists its render for non-commercial use. Keep these owner/source records with the assets and review distribution permissions before a commercial release.

| Asset | Published source | Character owner | Preparation |
| --- | --- | --- | --- |
| Remy portrait | [PNGkit character render](https://www.pngkit.com/view/u2y3a9t4y3w7r5q8_ratatouille-ratatouille-remy/) | Disney / Pixar | Exterior white matte removed; resized with aspect ratio preserved |
| Emile portrait | [PNGWing character render](https://www.pngwing.com/en/free-png-bmmxp) | Disney / Pixar | Exterior light checkerboard matte removed; aspect ratio preserved |
| Remy cheese / strawberry | [Remy Eating, Disney on GIPHY](https://giphy.com/gifs/PylzSrsqgmmqkisroH) | Disney / Pixar | Original film frames 0–13 / 14–27; original background retained |
| Emile snack | [Disney Pixar on GIPHY](https://giphy.com/gifs/jnkvKeUHWnRYs) | Disney / Pixar | Right-hand Emile region cropped from original film frames |
| Pikachu | [Pokémon on GIPHY](https://giphy.com/stickers/FCffpN404oRZpFbSzl) | The Pokémon Company | Original 3D animated sticker |
| Eevee | [Pokémon on GIPHY](https://giphy.com/stickers/Nk39jnb19T7MjcWTBi) | The Pokémon Company | Original 2D animated sticker |
| Bulbasaur | [Pokémon on GIPHY](https://giphy.com/stickers/eN6H6Eszm15OwKEONx) | The Pokémon Company | Original 3D animated sticker |
| Charmander | [Pokémon on GIPHY](https://giphy.com/stickers/6xr4bW2csldWKmJkjO) | The Pokémon Company | Original 3D animated sticker |
| Squirtle | [Pokémon on GIPHY](https://giphy.com/stickers/sIo7BCXxPDPNDfS3dE) | The Pokémon Company | Original 3D animated sticker |
| Jigglypuff | [Pokémon on GIPHY](https://giphy.com/stickers/70156xKexe2CqBOOSB) | The Pokémon Company | Original 3D animated sticker |
| Snoopy idle | [Peanuts on GIPHY](https://giphy.com/stickers/daOM5veK5Mwasb5TLY) | Peanuts Worldwide LLC | Snoopy region cropped from original animated sticker |
| Snoopy happy dance | [Peanuts on GIPHY](https://giphy.com/stickers/4bz6frJ6gNlCUEExIu) | Peanuts Worldwide LLC | Original animated sticker |
| Woodstock | [Peanuts on GIPHY](https://giphy.com/stickers/jptAHfCnH8rSgVSjcE) | Peanuts Worldwide LLC | Original animated sticker |
| Belle | [Peanuts on GIPHY](https://giphy.com/stickers/3UPF60aOXPYbpljc9m) | Peanuts Worldwide LLC | Belle cropped; exterior yellow matte/lettering removed; source ink and colors preserved |

Animated sources are converted to lossless WebP with source frame timing. Still selection/reduced-motion posters use the first prepared frame. `sources.json` records exact download URLs, source/output SHA-256 hashes, pixel sizes, frame counts, timing and transformations. `scripts/prepare-pet-assets.py` reproduces preparation with Pillow; normal builds never fetch assets.

Quiet variants select the opening neutral frames from the same sources, return through those original frames and dwell for four seconds. Their timing is four times slower. Pikachu's normal idle also uses that calm sequence, at twice the source duration with a shorter dwell; the original full animation is reserved for its occasional moves. `frames`, `timeScale`, `pingPong`, `holdMs` and `wholeSourceBounds` in the manifest document these adaptations. The shared source bounds preserve the character's scale and footing when switching modes. Character poses and styles stay sourced from the original frames.

Electricity, travel, silver sweep, petals, embers, bubbles and music particles are app-created effects layered over original character art. Volt Tackle and Iron Tail are interpretations using source character frames and transforms, not extracted official attack sequences. Remy/Emile movie actions retain their rectangular scene backgrounds in rounded inset clips, rather than inventing missing character poses. No audio is played.
