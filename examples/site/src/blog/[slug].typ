// Dynamic route: generates one page per blog entry.
#import "/lib/aster/content.typ": get-collection-ids, get-entry
#metadata(get-collection-ids("blog").map(id => (slug: id))) <route>

#let slug = sys.inputs.at("slug", default: "")
#let post = get-entry("blog", slug)
#if post != none {
  let rendered = post.render()
  html.html({
    html.head[
      #html.meta(charset: "utf-8")
      #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
      #html.title("Blog Post")
      #html.elem("link", attrs: ("rel": "css", "href": "../style.css"))
    ]
    html.body[
      = Blog Post
      #rendered.content
    ]
  })
}
