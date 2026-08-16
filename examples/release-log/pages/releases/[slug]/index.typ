#import "/lib.typ": get-entry, releases, route-param
#import "/components/release.typ": release
#import "/templates/site.typ": site

#metadata(releases().map(item => (slug: item.entry.id))) <aster-route>

#let entry = get-entry("releases", route-param("slug", default: ""))
#if entry != none [
  #let item = (entry: entry, metadata: entry.metadata())
  #show: site.with(title: item.metadata.title, description: item.metadata.description)
  #html.main[
    #release(item, entry.render(), linked: false, single: true)
  ]
]
