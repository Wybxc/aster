// Catch-all: renders any first-level slug not handled by static routes.
#import "/lib/aster/content.typ": get-collection-ids, get-entry
#metadata(
  get-collection-ids("blog").map(id => (slug: id))
    + ((slug: "about"),)
) <route>

#let slug = sys.inputs.at("slug", default: "")
#let post = get-entry("blog", slug)
#if post != none {
  let rendered = post.render()
  html.html({
    html.head[
      #html.meta(charset: "utf-8")
      #html.title("Blog Post")
    ]
    html.body[
      = Catch-All Page
      #raw(repr(rendered.metadata))
      #rendered.content
    ]
  })
} else if slug == "about" {
  html.html({
    html.head[#html.title("About")]
    html.body[
      = About
      This is a static-like page served via a catch-all route.
    ]
  })
}
