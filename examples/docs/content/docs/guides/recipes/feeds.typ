#import "/components/content.typ": callout

#metadata((
  title: "Feeds and Site Indexes",
  description: "Apply the generator interface to Atom, sitemap, robots, and other exact-path files.",
  section: "Guides",
  section_order: 20,
  order: 43,
)) <aster-frontmatter>

The #link("/reference/generators-and-postprocessing/")[Generators] reference
describes the generator interface. This recipe applies it to the concrete
output formats the examples produce.

= Atom feed

A generator runs after all pages have been transformed and receives the final
page snapshot through `_aster.site.pages`:

```typ
#import "@preview/exemel:0.1.0": to-xml
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

#let feed = (
  tag: "feed",
  attrs: (xmlns: "http://www.w3.org/2005/Atom"),
  children: (entry,),
)

#metadata(to-xml(feed)) <aster-output>
```

The exact Atom or RSS data model belongs to the project. The blog and journal
examples use the `exemel` Typst package to serialize an Atom dictionary. They
select the matching page from `_aster.site.pages` so the feed contains
transformed HTML rather than source content.

= Sitemap

A sitemap can use planned page paths directly:

```typ
#import "/lib.typ": absolute-page-url

#let paths = sys.inputs._aster.routes.pages()
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
#import "/lib.typ": settings

#let content = "User-agent: *\nAllow: /\n\nSitemap: "
  + settings.site.url + "sitemap.xml\n"
#metadata(content) <aster-output>
```

The generator file name determines the exact output path. No special feed,
sitemap, or text-output API is required.

#callout(kind: "note", title: "Final page content")[
  A page must label one main fragment with `<aster-content>` for a generator to
  receive `content.html` and `content.text`. The full `page.html` remains
  available for formats that need the complete document.
]
