#import "/templates/content.typ": post

#show: post.with(
  title: "Customizing AstroPaper theme color schemes",
  description: "Adjust the light and dark palettes without changing page components.",
  author: "Sat Naing",
  date: "2022-09-25T15:20:35Z",
  modified: "2026-05-17T04:57:06Z",
  tags: ("color-schemes", "docs"),
)

AstroPaper expresses its palette through a small set of semantic CSS variables.
Components refer to roles such as background, foreground, accent, and border,
so a new scheme does not require editing every selector. The complete palette
lives next to the site template in `templates/theme.css`.

= Light theme values

```css
:root,
[data-theme="light"] {
  --background: #fdfdfd;
  --foreground: #282728;
  --accent: #006cac;
  --muted: #e6e6e6;
  --border: #dedcdc;
}
```

= Dark theme values

Dark mode should choose a distinct background, readable foreground, and an
accent that remains visible for links and focus outlines.

```css
[data-theme="dark"] {
  --background: #212737;
  --foreground: #eaedf3;
  --accent: #ff6b01;
  --muted: #343f60;
  --border: #8f4c1c;
}
```

Check both schemes at mobile and desktop widths after changing the tokens.
