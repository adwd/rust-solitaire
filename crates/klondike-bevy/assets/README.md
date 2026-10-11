# Embedded artwork

The palette follows `klondike-egui/src/theme.rs`: felt `#104137`, paper
`#fffaef`, red `#b7303b`, black `#1e312e` and card back `#224b58`.

The card fronts in `source/` are adapted from Adrian Kennard (RevK)'s
[SVG playing cards](https://www.me.uk/cards/), released under
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
The source deck was generated using the medium index and plain ace options.
Changes here: reduced indices, a larger ace pip, the web game's paper and ink
colors, subdued court colors, and a light border. The blue-green back is an
original SVG matching the web frontend. The SVGs are included for editing.

Manrope and DM Serif Display are from Google Fonts. Their SIL Open Font License
texts and unmodified font files are in `fonts/`.

To regenerate (authoring tools only; never needed to run the game):

```sh
npm install --prefix crates/klondike-bevy/scripts
node crates/klondike-bevy/scripts/render_cards.cjs
python3 -m pip install Pillow
python3 crates/klondike-bevy/scripts/generate_assets.py
```

The game embeds the generated PNGs and fonts in the executable. `cards.png` is
an 8 × 7 atlas in C/D/H/S order, A through K, with 320 × 448-pixel cells.
