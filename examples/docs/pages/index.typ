#import "/lib.typ": get-entry
#import "/templates/docs.typ": docs-page

#let entry = get-entry("docs", "index")
#if entry != none { docs-page(entry) }
