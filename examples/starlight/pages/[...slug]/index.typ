#import "/lib.typ": get-collection-ids, get-entry, route-params
#import "/templates/docs.typ": docs-page

#metadata(
  get-collection-ids("docs")
    .filter(id => id != "index")
    .map(slug => (slug: slug))
) <aster-route>

#let slug = route-params.at("slug", default: "")
#let entry = get-entry("docs", slug)
#if entry != none { docs-page(entry) }
