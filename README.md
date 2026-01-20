# `oklch-pixel`

`oklch-pixel` generates a 1×1 PNG image given an OKLCH color.

You might want to use this program if you want to have a solid-color desktop background that’s outside the sRGB gamut but is in the wider-gamut Display P3 color space. Put another way, you can use this program to get colors beyond what you can specify with sRGB hex codes like `#efdfa2`.

If you want to pick a color in OKLCH, visit <https://oklch.com/>. This site is more interesting if you’re viewing it on a monitor that supports Display P3. The Apple Studio Display does this.

For example, if you want to punch up the old Windows Teal ([`#008080`, `oklch(0.5431 0.0927 194.77)`][old]) to something with more chroma (less gray) at the same lightness ([`oklch(0.5431 0.124 194.77)`][new-hotness]) and you have a monitor that supports Display P3, you can run

[old]:         https://oklch.com/#0.5431,0.0927,194.77,100
[new-hotness]: https://oklch.com/#0.5431,0.124,194.77,100

```sh
oklch-pixel 0.5431 0.0927 194.77
oklch-pixel 0.5431 0.124  194.77
```

and set the generated `oklch(0.5431 0.124 194.77).png` as your desktop wallpaper. If you want to flip back to the original, grayer color, set `oklch(0.5431 0.0927 194.77).png` as your background to compare.

## Bugs and limitations

- It works on my machine. I haven’t tested images with an alpha channel at all.
- This program can’t go beyond Display P3 into Rec2020.

## Humanity

To a first approximation, I don’t know Rust. Robots made this.
