#import "@preview/exemel:0.1.0": to-xml
#import "/lib.typ": route-pages, settings

#let paths = route-pages.filter(path => path != "/404.html").sorted()

#let sitemap = (
  tag: "urlset",
  attrs: (xmlns: "http://www.sitemaps.org/schemas/sitemap/0.9"),
  children: paths.map(path => (
    tag: "url",
    children: ((tag: "loc", children: (settings.site.url + path.slice(1),)),),
  )),
)

#metadata(to-xml(sitemap, pretty: true)) <aster-endpoint>
