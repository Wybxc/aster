#import "/components/content.typ": callout

#metadata((
  title: "Recipe: Feeds and Site Indexes",
  description: "Generate Atom, sitemap, robots, and other exact-path files from the final site.",
  section: "Guides",
  section_order: 20,
  order: 43,
)) <aster-frontmatter>

A generator runs after all pages have been transformed and receives the final
page snapshot through `_aster.site.pages`:

```typ
#import "/lib.typ": absolute-page-url, settings

#let pages = sys.inputs._aster.site.pages
#let page = pages.find(page => page.path == "/posts/hello/")

#let entry = (
  tag: "entry",
  children: (
    (tag: "title", children: ("Hello",)),
    (tag: "link", attrs: (href: absolute-page-url(page.path))),
    (tag: "content", attrs: (type: "html"), children: (page.content.html,)),
  ),
)

#metadata(to-xml(feed)) <aster-output>
```

The exact Atom or RSS data model belongs to the project. `page.html` is the
complete final page before external postprocessing; `page.content.html` and
`page.content.text` come from the page's unique `<aster-content>` fragment.

The blog and journal examples use the `exemel` Typst package to serialize an
Atom dictionary. Their generators also select the matching page from
`_aster.site.pages` so the feed contains transformed HTML rather than source
content.

= Sitemap

A sitemap can use planned page paths directly:

```typ
#let paths = sys.inputs._aster.routes.pages
#let urls = paths
  .filter(path => path != "/404.html")
  .map(path => (
    tag: "url",
    children: ((tag: "loc", children: (absolute-page-url(path),)),),
  ))
```

Wrap `urls` in the XML structure required by the sitemap format and emit it
from a file such as `generate/sitemap.xml.typ`.

= Robots

`robots.txt` is just a string generator:

```typ
#let content = "User-agent: *\nAllow: /\n\nSitemap: "
  + settings.site.url + "sitemap.xml\n"
#metadata(content) <aster-output>
```

The generator file name determines the exact output path. No special feed,
sitemap, or text-output API is required.

= Choose the phase

Use a page when the result is part of the navigable site. Use a generator when
the result is an exact-path file derived from routes or final page HTML. Use a
postprocessor when an external program needs the complete staged filesystem,
such as a search indexer or deployment-specific optimizer.

#callout(kind: "note", title: "Final page content")[
  A page must label one main fragment with `<aster-content>` for a generator to
  receive `content.html` and `content.text`. The full `page.html` remains
  available for formats that need the complete document.
]
