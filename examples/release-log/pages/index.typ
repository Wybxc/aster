#import "/lib.typ": releases
#import "/components/release.typ": release
#import "/templates/site.typ": site

#show: site

#html.main[
  #html.h1(class: "page-title")[Changelog]
  #html.hr()
  #html.div(class: "releases")[
    #for item in releases() {
      release(item, item.entry.render())
    }
  ]
]
