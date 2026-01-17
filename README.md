# `oklch-pixel`

`oklch-pixel` generates a pixel given an OKLCH color.

You might want to use this program if you want to have a solid-color desktop background that’s outside the sRGB gamut — that is, colors you can't specify with sRGB hex codes like `#efdfa2`.

If you want to pick a color in OKLCH, visit <https://oklch.com/>. This site is more interesting if you’re viewing it on a monitor that supports Display P3 (a wider-gamut color space than sRGB). The Apple Studio Display does this.

For example, if you want to punch up the old Windows Teal (`#008080`, `oklch(0.5431 0.0927 194.77)`) to something with more chroma (less gray) at the same lightness (`oklch(0.5431 0.124 194.77)`) and you have a monitor that supports Display P3, you can run

```sh
oklch-pixel 0.5431 0.0927 194.77
oklch-pixel 0.5431 0.124  194.77
```

and set the generated `oklch(0.5431 0.124 194.77).png` as your desktop wallpaper. If you want to flip back to the original, grayer color, set `oklch(0.5431 0.0927 194.77).png` as your background to compare.

As I’m writing this README, it’s 2026, and I assume you don’t have a monitor that supports the even-wider Rec2020. If you do, you’ll want to pick a color even further away from sRGB’s limitations to impress your friends and terrify your enemies.

## Bugs

It works on my machine. I haven’t tested images with an alpha channel at all.

## Humanity

To a first approximation, I don’t know Rust. Robots made this.
