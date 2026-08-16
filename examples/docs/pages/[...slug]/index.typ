#import "/lib.typ": get-collection-ids, get-entry, route-param
#import "/templates/docs.typ": docs-page

#metadata(
  get-collection-ids("docs")
    .filter(id => id != "index")
    .map(slug => (slug: slug))
) <aster-route>

#let slug = route-param("slug", default: "")
#let entry = get-entry("docs", slug)
#if entry != none { docs-page(entry) }
