// Dynamic route: generates one page per blog entry.
#import "/lib/aster/content.typ": get-collection, get-entry, render
#metadata(get-collection("blog").map(e => (slug: e.id))) <route>

#let slug = sys.inputs.at("slug", default: "")
#let post = get-entry("blog", slug)
#if post != none {
  html.html({
    html.head[
      #html.meta(charset: "utf-8")
      #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
      #html.title("Blog Post")
      #html.elem("link", attrs: ("rel": "css", "href": "style.css"))
    ]
    html.body[
      = Blog Post
      #render(post)
    ]
  })
}
